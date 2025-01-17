#![no_std]
use soroban_env_common::BytesObject;
use soroban_sdk::{contract, contractimpl, Bytes, Env, FromVal, Val, Vec};

#[contract]
pub struct Contract;

// The Ackermann function
fn ack(m: u32, n: u32) -> u32 {
    if m == 0 {
        return n + 1;
    }
    if n == 0 {
        return ack(m - 1, 1);
    }
    return ack(m - 1, ack(m, n - 1));
}

fn stacksmash(n: u64) {
    if n > 0 {
        stacksmash(n + n / 2)
    }
}

static GLOBAL_BUF: [u32; 10] = [0; 10];

// We use a pub mutable global here so the optimizer
// doesn't get clever and try to prove the OOB access
// is certain and reduce it to a single `unreachable`.
pub static mut BAD_IDX: usize = 100;

#[contractimpl]
impl Contract {
    pub fn iloop() {
        loop {}
    }

    pub fn badack() {
        ack(4, 8);
    }

    pub fn ssmash() {
        stacksmash(2)
    }

    pub fn oob1() -> u32 {
        GLOBAL_BUF[unsafe { BAD_IDX }] + 1
    }

    pub fn oob2() -> u32 {
        let local_buf: [u32; 10] = [0; 10];
        local_buf[unsafe { BAD_IDX }] + 1
    }

    pub fn objs(env: Env) {
        let local_buf: [u8; 1024] = [0; 1024];
        for _ in 0..100000 {
            Bytes::from_slice(&env, &local_buf);
        }
    }

    pub fn forgeref(env: Env, lo: u32, hi: u32) -> u32 {
        let payload: u64 = lo as u64 | ((hi as u64) << 32);
        let v: Vec<u32> = Vec::from_val(&env, &Val::from_payload(payload));
        v.get(0).unwrap()
    }

    // Forge a type and call a method on it.
    pub fn forgety1(env: Env, v: Vec<u32>) -> u32 {
        let b = Bytes::from_val(
            &env,
            &unsafe { BytesObject::from_handle(v.to_object().get_handle()) }.to_val(),
        );
        b.get(0).unwrap() as u32
    }

    // Forge a type and pass it as an argument.
    #[allow(unused_mut)]
    pub fn forgety2(env: Env, mut v: Vec<Bytes>) {
        let b = Bytes::from_val(
            &env,
            &unsafe { BytesObject::from_handle(v.to_object().get_handle()) }.to_val(),
        );
        v.push_back(b)
    }

    // Pass a value with a bad tag.
    #[allow(unused_mut)]
    pub fn badtag(_env: Env, mut v: Vec<Val>) {
        let bad = Val::from_payload((v.to_val().get_payload() & !0xff) | 32);
        v.push_back(bad)
    }
}
