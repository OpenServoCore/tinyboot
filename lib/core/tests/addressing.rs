//! Addressable devices sharing one simulated wire: a Select walk finds
//! every id, and a device speaks only when selected.

mod common;

use common::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use tinyboot_core::addressing::{Addressable, Addressing};
use tinyboot_core::app::{App, AppConfig};
use tinyboot_core::protocol::Dispatcher;
use tinyboot_core::traits::BootState;
use tinyboot_protocol::crc::{CRC_INIT, crc16};
use tinyboot_protocol::frame::{DEFAULT_PAYLOAD, Flags, Frame};
use tinyboot_protocol::{Cmd, ResetFlags, Status};

// This bootloader's image tail: no compatibility key.
tinyboot_core::boot_version!();

type Uid = fn() -> &'static [u8; 12];
type Dev<'a> = Dispatcher<
    'a,
    MockTransport,
    MockStorage,
    MockBootMeta,
    MockBootCtl,
    TEST_BUF_SIZE,
    DEFAULT_PAYLOAD,
    Addressable<12, Uid>,
>;

/// Ids sharing long prefixes, so the walk descends deep: 0 and 1 differ
/// only in their last bit, 2 and 3 in their first.
static UIDS: [[u8; 12]; 6] = [
    [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, 0x01, 0x23, 0x45, 0x07,
    ],
    [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, 0x01, 0x23, 0x45, 0x87,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ],
    [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0x7E, 0x01, 0x23, 0x45, 0x07,
    ],
];

fn uid<const K: usize>() -> &'static [u8; 12] {
    &UIDS[K]
}

const UID_FNS: [Uid; 6] = [uid::<0>, uid::<1>, uid::<2>, uid::<3>, uid::<4>, uid::<5>];

fn platforms(n: usize) -> Vec<TestPlatform> {
    (0..n).map(|_| platform(BootState::Idle)).collect()
}

fn devices<'a>(ps: &'a mut [TestPlatform], uids: &[Uid]) -> Vec<Dev<'a>> {
    ps.iter_mut()
        .zip(uids)
        .map(|(p, &u)| Dispatcher::with_addressing(p, Addressable::new(u)))
        .collect()
}

/// Load a frame as the host or another device would send it.
fn load(t: &mut MockTransport, cmd: Cmd, status: Status, flags: u8, addr: u32, data: &[u8]) {
    let mut f = Frame::<DEFAULT_PAYLOAD>::default();
    f.cmd = cmd;
    f.status = status;
    f.set_addr(addr);
    f.flags = Flags { raw: flags };
    f.len = data.len() as u16;
    unsafe { f.data.raw[..data.len()].copy_from_slice(data) };
    let mut tmp = MockTransport::new();
    f.send(&mut tmp).unwrap();
    t.rx_buf[..tmp.tx_pos].copy_from_slice(&tmp.tx_buf[..tmp.tx_pos]);
    t.rx_len = tmp.tx_pos;
    t.rx_pos = 0;
    t.tx_pos = 0;
    t.waited = None;
}

/// The device's reply, read back as a host would; `None` if it was silent.
fn reply(t: &MockTransport) -> Option<Frame> {
    if t.tx_pos == 0 {
        return None;
    }
    let mut wire = MockTransport::new();
    wire.rx_buf[..t.tx_pos].copy_from_slice(&t.tx_buf[..t.tx_pos]);
    wire.rx_len = t.tx_pos;
    let mut f = Frame::default();
    assert_eq!(f.read(&mut wire).unwrap(), Status::Ok);
    assert_eq!(wire.rx_pos, wire.rx_len, "trailing bytes after the reply");
    Some(f)
}

/// `Select` data for a prefix of `bits` (bit k = id[k / 8] >> (k % 8) & 1).
fn select_data(seed: u8, bits: &[bool]) -> Vec<u8> {
    let mut d = vec![seed, bits.len() as u8];
    d.resize(2 + bits.len().div_ceil(8), 0);
    for (k, &b) in bits.iter().enumerate() {
        d[2 + k / 8] |= (b as u8) << (k % 8);
    }
    d
}

fn bits_of(uid: &[u8]) -> Vec<bool> {
    (0..uid.len() * 8)
        .map(|k| uid[k / 8] >> (k % 8) & 1 != 0)
        .collect()
}

enum Heard {
    Nothing,
    Clean(Frame),
    Garble,
}

/// Send one host request to every device; replies from two or more at once
/// garble, as on a real wire.
fn exchange(devs: &mut [Dev], cmd: Cmd, flags: u8, addr: u32, data: &[u8]) -> Heard {
    let mut heard = Vec::new();
    for d in devs.iter_mut() {
        load(
            &mut d.platform.transport,
            cmd,
            Status::Request,
            flags,
            addr,
            data,
        );
        d.dispatch().unwrap();
        heard.extend(reply(&d.platform.transport));
    }
    match heard.len() {
        0 => Heard::Nothing,
        1 => Heard::Clean(heard.pop().unwrap()),
        _ => Heard::Garble,
    }
}

fn select(devs: &mut [Dev], seed: u8, bits: &[bool]) -> Heard {
    exchange(devs, Cmd::Select, 0, 0, &select_data(seed, bits))
}

fn uid_of(f: &Frame) -> Vec<u8> {
    unsafe { f.data.raw[..f.len as usize].to_vec() }
}

fn walk(devs: &mut [Dev], prefix: &mut Vec<bool>, seed: &mut u8, found: &mut Vec<Vec<u8>>) {
    *seed = seed.wrapping_add(1);
    match select(devs, *seed, prefix) {
        Heard::Nothing => {}
        Heard::Clean(f) => {
            let id = uid_of(&f);
            assert!(bits_of(&id).starts_with(prefix), "reply outside the prefix");
            found.push(id);
        }
        Heard::Garble => {
            assert!(prefix.len() < 96, "two devices with one id");
            for b in [false, true] {
                prefix.push(b);
                walk(devs, prefix, seed, found);
                prefix.pop();
            }
        }
    }
}

#[test]
fn select_walk_finds_every_uid() {
    let mut ps = platforms(UIDS.len());
    let mut devs = devices(&mut ps, &UID_FNS);
    let mut found = Vec::new();
    walk(&mut devs, &mut Vec::new(), &mut 0, &mut found);
    found.sort();
    let mut want: Vec<Vec<u8>> = UIDS.iter().map(|u| u.to_vec()).collect();
    want.sort();
    assert_eq!(found, want);
}

#[test]
fn full_uid_selects_one_and_deselects_the_rest() {
    let mut ps = platforms(3);
    let mut devs = devices(&mut ps, &UID_FNS[..3]);

    for k in [1, 0, 2] {
        let Heard::Clean(f) = select(&mut devs, 7, &bits_of(&UIDS[k])) else {
            panic!("select of device {k} not answered by one device");
        };
        assert_eq!(f.status, Status::Ok);
        assert_eq!(uid_of(&f), UIDS[k]);
        for (j, d) in devs.iter().enumerate() {
            assert_eq!(d.addressing.selected(), j == k);
        }
        let Heard::Clean(f) = exchange(&mut devs, Cmd::Info, 0, 0, &[]) else {
            panic!("info not answered by one device");
        };
        assert_eq!(f.status, Status::Ok);
    }
}

#[test]
fn empty_prefix_selects_all_and_too_long_a_prefix_none() {
    let mut ps = platforms(3);
    let mut devs = devices(&mut ps, &UID_FNS[..3]);

    assert!(matches!(select(&mut devs, 0, &[]), Heard::Garble));
    assert!(devs.iter().all(|d| d.addressing.selected()));

    // 97 bits: longer than any id, so it deselects everyone.
    let data = [0, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert!(matches!(
        exchange(&mut devs, Cmd::Select, 0, 0, &data),
        Heard::Nothing
    ));
    assert!(devs.iter().all(|d| !d.addressing.selected()));
}

#[test]
fn unselected_devices_never_transmit() {
    let mut ps = platforms(3);
    let mut devs = devices(&mut ps, &UID_FNS[..3]);

    for cmd in [Cmd::Info, Cmd::Erase, Cmd::Write, Cmd::Verify, Cmd::Reset] {
        assert!(matches!(
            exchange(&mut devs, cmd, 0, 0, &[0, 0]),
            Heard::Nothing
        ));
    }
    // A prefix matching none of them.
    let mut bits = bits_of(&UIDS[0]);
    bits[94] = !bits[94];
    assert!(matches!(select(&mut devs, 0, &bits), Heard::Nothing));

    for d in devs.iter_mut() {
        let t = &mut d.platform.transport;
        // Bad CRC.
        load(t, Cmd::Info, Status::Request, 0, 0, &[]);
        t.rx_buf[t.rx_len - 1] ^= 1;
        d.dispatch().unwrap();
        assert!(reply(&d.platform.transport).is_none());
        // Unknown command.
        let t = &mut d.platform.transport;
        load(t, Cmd::Info, Status::Request, 0, 0, &[]);
        t.rx_buf[2] = 0x06;
        d.dispatch().unwrap();
        assert!(reply(&d.platform.transport).is_none());
        // Payload overflow.
        let t = &mut d.platform.transport;
        load(t, Cmd::Info, Status::Request, 0, 0, &[]);
        t.rx_buf[8..10].copy_from_slice(&(DEFAULT_PAYLOAD as u16 + 1).to_le_bytes());
        d.dispatch().unwrap();
        assert!(reply(&d.platform.transport).is_none());
    }
}

#[test]
fn selected_device_ignores_bad_frames_and_other_replies() {
    let mut ps = platforms(1);
    let mut devs = devices(&mut ps, &UID_FNS[..1]);
    assert!(matches!(select(&mut devs, 0, &[]), Heard::Clean(_)));
    let d = &mut devs[0];

    let t = &mut d.platform.transport;
    load(t, Cmd::Info, Status::Request, 0, 0, &[]);
    t.rx_buf[t.rx_len - 1] ^= 1;
    d.dispatch().unwrap();
    assert!(reply(&d.platform.transport).is_none());

    // Another device's Info reply on the shared wire.
    load(
        &mut d.platform.transport,
        Cmd::Info,
        Status::Ok,
        0,
        0,
        &[0; 18],
    );
    d.dispatch().unwrap();
    assert!(reply(&d.platform.transport).is_none());
    assert!(d.addressing.selected());
}

/// Two ids drawing the same slot under seed 0.
fn colliding_pair() -> &'static [[u8; 12]; 2] {
    static PAIR: OnceLock<[[u8; 12]; 2]> = OnceLock::new();
    PAIR.get_or_init(|| {
        let id = |k: u8| [k, 0x5A, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x33];
        let slot = |u: &[u8]| crc16(CRC_INIT, u) & 15;
        let b = (1..=255).find(|&k| slot(&id(k)) == slot(&id(0))).unwrap();
        [id(0), id(b)]
    })
}

fn pair0() -> &'static [u8; 12] {
    &colliding_pair()[0]
}

fn pair1() -> &'static [u8; 12] {
    &colliding_pair()[1]
}

#[test]
fn unison_replies_are_split_by_the_seed() {
    let mut ps = platforms(2);
    let mut devs = devices(&mut ps, &[pair0, pair1]);
    let mut split = false;
    for seed in 0..=255u8 {
        assert!(matches!(select(&mut devs, seed, &[]), Heard::Garble));
        let w: Vec<u8> = devs
            .iter()
            .map(|d| d.platform.transport.waited.unwrap())
            .collect();
        assert!(w.iter().all(|&n| n < 16));
        if seed == 0 {
            assert_eq!(w[0], w[1], "the pair should collide under seed 0");
        }
        split |= w[0] != w[1];
    }
    assert!(split, "no seed separates the pair");
}

#[test]
fn reset_reply_is_flushed_before_the_reset() {
    let mut ps = platforms(1);
    let mut devs = devices(&mut ps, &UID_FNS[..1]);
    assert!(matches!(select(&mut devs, 0, &[]), Heard::Clean(_)));
    let d = &mut devs[0];
    load(
        &mut d.platform.transport,
        Cmd::Reset,
        Status::Request,
        ResetFlags::BOOTLOADER.bits(),
        0,
        &[],
    );
    let flushes = d.platform.transport.flushes;
    assert!(catch_unwind(AssertUnwindSafe(|| d.dispatch())).is_err());
    let f = reply(&d.platform.transport).expect("no Reset reply");
    assert_eq!(f.status, Status::Ok);
    assert_eq!(d.platform.transport.flushes, flushes + 1);
}

#[test]
fn unaddressed_bootloader_refuses_select() {
    let mut p = platform(BootState::Idle);
    let mut d = Dispatcher::<_, _, _, _, TEST_BUF_SIZE>::new(&mut p);
    load(
        &mut d.platform.transport,
        Cmd::Select,
        Status::Request,
        0,
        0,
        &[0, 0],
    );
    d.dispatch().unwrap();
    assert_eq!(
        reply(&d.platform.transport).unwrap().status,
        Status::Unsupported
    );
}

static APP_WAITED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0xFF);

fn app_wait(n: u8) {
    APP_WAITED.store(n, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn addressable_app_answers_only_once_selected() {
    let config = AppConfig {
        capacity: 1024,
        erase_size: 64,
        boot_version: 1,
        boot_compat: 0,
        app_version: 2,
    };
    let mut app: App<MockBootCtl, MockBootMeta, DEFAULT_PAYLOAD, _> = App::with_addressing(
        config,
        MockBootCtl,
        MockBootMeta::new(BootState::Idle),
        Addressable::new(UID_FNS[3]),
        app_wait,
    );
    let mut rx = MockTransport::new();
    let mut tx = MockTransport::new();

    load(&mut rx, Cmd::Info, Status::Request, 0, 0, &[]);
    app.poll(&mut rx, &mut tx);
    assert!(reply(&tx).is_none());

    load(
        &mut rx,
        Cmd::Select,
        Status::Request,
        0,
        0,
        &select_data(3, &bits_of(&UIDS[3])),
    );
    app.poll(&mut rx, &mut tx);
    let f = reply(&tx).expect("no Select reply");
    assert_eq!(uid_of(&f), UIDS[3]);
    assert!(APP_WAITED.load(std::sync::atomic::Ordering::Relaxed) < 16);

    tx.tx_pos = 0;
    load(&mut rx, Cmd::Info, Status::Request, 0, 0, &[]);
    app.poll(&mut rx, &mut tx);
    let f = reply(&tx).expect("no Info reply");
    assert_eq!(unsafe { f.data.info }.mode, 1);
}
