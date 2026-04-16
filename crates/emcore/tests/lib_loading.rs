//! Behavioral tests for the dynamic library API (emTryOpenLib, etc.)
//!
//! These tests load real .so files to verify the full dlopen path.

use emcore::emStd2::{
    emCloseLib, emTryOpenLib, emTryResolveSymbol, emTryResolveSymbolFromLib, lib_name_to_filename,
};

#[test]
fn open_nonexistent_returns_error() {
    let result = emTryOpenLib("this_library_does_not_exist_xyz", false);
    assert!(result.is_err());
}

#[test]
fn filename_construction_linux() {
    assert_eq!(lib_name_to_filename("emStocks"), "libemStocks.so");
    assert_eq!(lib_name_to_filename("emCore"), "libemCore.so");
}

#[test]
fn open_libc_and_resolve_symbol() {
    // libc.so.6 is always available on Linux
    let handle = emTryOpenLib("libc.so.6", true).expect("should open libc");
    let ptr = unsafe { emTryResolveSymbolFromLib(&handle, "strlen").expect("should find strlen") };
    assert!(!ptr.is_null());
    emCloseLib(handle);
}

#[test]
fn resolve_symbol_sets_infinite_lifetime() {
    let ptr =
        unsafe { emTryResolveSymbol("libc.so.6", true, "strlen").expect("should resolve strlen") };
    assert!(!ptr.is_null());
    // Library now has infinite lifetime — closing is a no-op
}

#[test]
fn resolve_nonexistent_symbol_returns_error() {
    let handle = emTryOpenLib("libc.so.6", true).expect("should open libc");
    let result = unsafe { emTryResolveSymbolFromLib(&handle, "this_does_not_exist_xyz") };
    assert!(result.is_err());
    emCloseLib(handle);
}
