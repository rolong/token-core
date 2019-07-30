use std::ffi::{CString, CStr};
use libc::{size_t, c_int};
use std::os::raw::{c_char, c_void};
use log::Level;
use log::{info, trace, warn};

use std::fs::File;
use std::io::{Read, Write};
use utils::Result;
use utils::LAST_BACKTRACE;
use utils::LAST_ERROR;
use failure::Fail;

use crate::utils::landingpad;

use serde_json::Value;
use tcx_chain::{Metadata, Keystore, V3Keystore, HdKeystore};


use std::path::Path;
use std::collections::HashMap;
use tcx_chain::signer::TransactionSinger;
use std::fs::{self, DirEntry};

use std::rc::Rc;
use std::cell::RefCell;
use core::borrow::{BorrowMut, Borrow};
use serde_json::map::Keys;
use std::sync::Mutex;
use crate::utils::set_panic_hook;
use tcx_bch::bch_transaction::{Utxo, BitcoinCashTransaction};
use tcx_bch::bch_coin::{BchCoin, BchAddress};
use tcx_chain::curve::Secp256k1Curve;
use tcx_chain::coin::Coin;


// #[link(name = "TrezorCrypto")]
// extern {
//     fn mnemonic_generate(strength: c_int, mnemonic: *mut c_char) -> c_int;
// }
//pub mod utils;

#[macro_use]
extern crate failure;

#[macro_use]
pub mod utils;

#[macro_use]
extern crate lazy_static;

static PASSWORD: &'static str = "Insecure Pa55w0rd";
static MNEMONIC: &'static str = "inject kidney empty canal shadow pact comfort wife crush horse wife sketch";
static ETHEREUM_PATH: &'static str = "m/44'/60'/0'/0/0";

lazy_static! {
    static ref KEYSTORE_MAP: Mutex<HashMap<String, HdKeystore>> = {
        let mut m = Mutex::new(HashMap::new());
        m
    };

}

fn cache_keystore(keystore: HdKeystore) {
    KEYSTORE_MAP.lock().unwrap().insert(keystore.id.to_owned(), keystore);
}


fn find_keystore_id_by_address(address: &str) -> Option<String> {
    let map = KEYSTORE_MAP.lock().unwrap();
    let mut k_id: Option<String> = None;
    for (id, keystore) in map.borrow().iter() {
        let mut iter = keystore.active_accounts.iter();
        if iter.find(|a| a.address == address).is_some() {
            k_id = Some(id.to_string());
            break;
        }
    }
    k_id

}

#[no_mangle]
pub extern fn read_file(file_path: *const c_char) -> *const c_char {
    let c_str = unsafe { CStr::from_ptr(file_path) };
    let file_path = c_str.to_str().unwrap();
    // let filePath: String = env.get_string(filePath).expect("Couldn't get java string!").into();
    let mut file = File::open(file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents);

    CString::new(contents).unwrap().into_raw()
}

#[no_mangle]
pub extern fn free_string(s: *mut c_char) {
    unsafe {
        if s.is_null() { return; }
        CString::from_raw(s)
    };
}

#[no_mangle]
pub extern fn free_const_string(s: *const c_char) {
    unsafe {
        if s.is_null() { return; }
        CStr::from_ptr(s)
    };
}

#[no_mangle]
pub unsafe extern "C" fn read_file_error() -> *const c_char {
    crate::utils::landingpad(||
        {
            Err(format_err!("{}", "read file error"))
        })
}


fn parse_arguments(json_str: *const c_char) -> Value {
    let json_c_str = unsafe { CStr::from_ptr(json_str) };
    let json_str = json_c_str.to_str().unwrap();
    serde_json::from_str(json_str).unwrap()
}


pub unsafe extern "C" fn create_wallet(password: *const c_char) -> *const c_char {
    let password_c_str = CStr::from_ptr(password);
    let password = password_c_str.to_str().unwrap();
    let json = landingpad(|| _create_wallet(password));
    CString::new(json).unwrap().into_raw()
}

fn _create_wallet(password: &str) -> Result<String> {
    let keystore = HdKeystore::new(password);
    let json = keystore.json();
    cache_keystore(keystore);
    Ok(json)
}

#[no_mangle]
pub unsafe extern "C" fn scan_wallets(json_str: *const c_char) {
    let json_c_str = unsafe { CStr::from_ptr(json_str) };
    let json_str = json_c_str.to_str().unwrap();
    let v: Value = serde_json::from_str(json_str).unwrap();
    set_panic_hook();
    _scan_wallets(v);
        ()
}

fn _scan_wallets(v: Value) -> Result<()> {
    let file_dir = v["fileDir"].as_str().unwrap();
    let p = Path::new(file_dir);
    let walk_dir = std::fs::read_dir(p).unwrap();
    for entry in walk_dir {
        let entry = entry.unwrap();
        let fp = entry.path();
        let mut f = File::open(fp).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents);
        let v: Value = serde_json::from_str(&contents).unwrap();

        let version = v["version"].as_i64().unwrap();
        if version != HdKeystore::VERSION as i64 {
            continue;
        }
        let keystore: HdKeystore = serde_json::from_str(&contents)?;
        cache_keystore(keystore);
    }
    Ok(())
}


#[no_mangle]
pub unsafe extern "C" fn find_wallet_by_mnemonic(json_str: *const c_char) -> *const c_char {
    let v = parse_arguments(json_str);
    let json = crate::utils::landingpad(|| _find_wallet_by_mnemonic(&v));
    CString::new(json).unwrap().into_raw()
}


fn _find_wallet_by_mnemonic(v: &Value) -> Result<String> {
    let mnemonic = v["mnemonic"].as_str().unwrap();
    let path = v["path"].as_str().unwrap();
    let network = v["network"].as_str().unwrap();
    let chain_type = v["chainType"].as_str().unwrap();
    // todo: provider support
    let password = "InsecurePassword";
    let ks = HdKeystore::from_mnemonic(mnemonic, password);
    let acc = match chain_type {
        "BCH" => BchCoin::<Secp256k1Curve, BchAddress>::mnemonic_to_account(mnemonic, path),
        _ => Err(format_err!("{}", "chain_type_not_support"))
    }?;
    let address = acc.address;
    let kid= find_keystore_id_by_address(&address);
    if let Some(id) = kid {
        let map =  KEYSTORE_MAP.lock().unwrap();
        let ks = map.get(&id).unwrap();
        Ok(ks.json())
    } else {
        Ok("{}".to_owned())
    }
}

#[no_mangle]
pub unsafe extern "C" fn import_wallet_from_mnemonic(json_str: *const c_char) -> *const c_char {
    let json_c_str = unsafe { CStr::from_ptr(json_str) };
    let json_str = json_c_str.to_str().unwrap();
    let json = crate::utils::landingpad(|| _import_wallet_from_mnemonic(json_str));
    CString::new(json).unwrap().into_raw()
}


fn _import_wallet_from_mnemonic(json_str: &str) -> Result<String> {
    let v: Value = serde_json::from_str(json_str).unwrap();

    let mut meta: Metadata = serde_json::from_value(v.clone())?;
    let password = v["password"].as_str().unwrap();
    let mnemonic = v["mnemonic"].as_str().unwrap();
    let path = v["path"].as_str().unwrap();
    let overwrite = v["overwrite"].as_bool().unwrap();
    let file_dir = v["fileDir"].as_str().unwrap();
    let chain_type = v["chainType"].as_str().unwrap();

    let mut ks = HdKeystore::from_mnemonic(mnemonic, password);
    let coin = match chain_type {
        "BCH" => {
            BchCoin::<Secp256k1Curve, BchAddress>::append_account(&mut ks, password, path)
        }
        _ => Err(format_err!("{}", "chain_type_not_support"))
    }?;


//    let mut keystore = HdMnemonicKeystore::new(meta, password, mnemonic, path)?;
    let account = coin.account();
    let exist_kid_opt = find_keystore_id_by_address(&account.address);
    if exist_kid_opt.is_some() {
        if !overwrite {
            return Err(format_err!("{}", "wallet_exists"));
        } else {
            ks.id = exist_kid_opt.unwrap();
        }
    }

    let json = ks.json();

    let ks_path = format!("{}{}.json", file_dir, ks.id);
    let path = Path::new(&ks_path);
    let mut file = File::create(path).unwrap();
    file.write_all(&json.as_bytes());


    cache_keystore(ks);
    Ok(json)
}


#[no_mangle]
pub unsafe extern "C" fn sign_transaction(json_str: *const c_char) -> *const c_char {
    let json_c_str = unsafe { CStr::from_ptr(json_str) };
    let json_str = json_c_str.to_str().unwrap();

    let json = crate::utils::landingpad(|| _sign_transaction(json_str));
    CString::new(json).unwrap().into_raw()
}

fn _sign_transaction(json_str: &str) -> Result<String> {
    let v: Value = serde_json::from_str(json_str).unwrap();
    let w_id = v["id"].as_str().unwrap();
    let chain_type = v["chainType"].as_str().unwrap();

    let mut map = KEYSTORE_MAP.lock().unwrap();
    let keystore = match map.get_mut(&w_id.to_owned()) {
        Some(keystore) => Ok(keystore),
        _ => Err(format_err!("{}", "wallet_id_not_found"))
    }?;

    match chain_type {
        "BCH" => {
            let coin = BchCoin::<Secp256k1Curve, BchAddress>::load(&keystore)?;
            coin.sign_transaction(json_str)
        }
        _ => Err(format_err!("{}", "chain_type_not_support"))
    }
}


#[no_mangle]
pub unsafe extern "C" fn clear_err() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
    LAST_BACKTRACE.with(|e| {
        *e.borrow_mut() = None;
    });
}

#[no_mangle]
pub unsafe extern "C" fn get_last_err_message() -> *const c_char {
    use std::fmt::Write;
    use std::error::Error;
    LAST_ERROR.with(|e| {
        if let Some(ref err) = *e.borrow() {
            let mut msg = err.to_string();
            // todo: follow cause
//            let mut cause = err.cause();
//            while let Some(the_cause) = cause {
//                write!(&mut msg, "\n  caused by: {}", the_cause).ok();
//                cause = &the_cause.cause();
//            }
            CString::new(msg).unwrap().into_raw()
        } else {
            CString::new("").unwrap().into_raw()
        }
    })
}


#[cfg(test)]
mod tests {
    use crate::import_wallet_from_mnemonic;
    use std::ffi::{CString, CStr};

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    //    #[test]
//    unsafe fn import_wallet() {
//        let data = r#"
//        {
//            "password": "PASSWORD",
//            "mnemonic": "inject kidney empty canal shadow pact comfort wife crush horse wife sketch",
//            "path": "m/44'/145'/0'",
//            "overwrite": false,
//            "name": "bch-ios",
//            "passwordHint": "",
//            "chainType": "BCH",
//            "network": "MAINNET",
//            "fileDir": "/tmp/imtoken/wallets"
//
//        }"#;
//        let json_str = CString::new(data).unwrap().into_raw();
//        let ret = unsafe { import_wallet_from_mnemonic(json_str)};
//        assert_eq!("", CStr::from_ptr(ret).to_str().unwrap());
//    }
    #[test]
    fn path() {
        let file_dir = "/Users/xyz/Library/Developer/CoreSimulator/Devices/1C6326AE-C550-43D5-A1A7-CF791B4A04CA/data/Containers/Data/Application/BC076852-DF07-42EA-82B1-2FA8C5CEE9EE/Documents/wallets/";
        let id = "ec9298f7-7f2b-4483-90af-cc440a411d82";

        let a_str = String::from("aaa");

        let ks_path = format!("{}{}.json", file_dir, id);
        assert_eq!("", ks_path);
    }
}
