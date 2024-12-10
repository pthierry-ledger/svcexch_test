// SPDX-FileCopyrightText: 2023 Ledger SAS
// SPDX-License-Identifier: Apache-2.0

use core::ptr::addr_of_mut;

const EXCHANGE_AREA_LEN: usize = 128; // TODO: replace by CONFIG-defined value

#[unsafe(link_section = ".svcexchange")]
static mut EXCHANGE_AREA: [u8; EXCHANGE_AREA_LEN] = [0u8; EXCHANGE_AREA_LEN];

pub enum Status {
    Ok,
    Invalid,
}

//struct ShmInfo {
//    handle: u32,
//    label: u32,
//    base: usize,
//    len: usize,
//    perms: u32,
//}

struct Area {
}

/// SVC Exchange area len. TODO: to be kconfig-generated

/// SVC Exchange exchange header size

pub trait ExhangeArea<T : ?Sized> {

    /// copy data to Exchange zone. length defines the number of T-typed data
    fn copy_to(&self, _from: *const T, _length: usize) -> Status {
        Status::Invalid
    }
    fn copy_from(&self, _from: *mut T, _length: usize) -> Status {
        Status::Invalid
    }
    fn area_length(&self) -> usize {
        EXCHANGE_AREA_LEN
    }
}

//impl ExhangeArea<ShmInfo> for Area {
//}

impl ExhangeArea<u8> for Area {

    #[allow(static_mut_refs)]
    fn copy_to(&self, from: *const u8, length: usize) -> Status {
        unsafe {
            if Area::check_overlapping(from, length).is_err() {
                return Status::Invalid;
            }
            core::ptr::copy_nonoverlapping(
                from,
                EXCHANGE_AREA.as_mut_ptr(),
                length.min(EXCHANGE_AREA_LEN),
            );
        }
        Status::Ok
    }

    #[allow(static_mut_refs)]
    fn copy_from(&self, to: *mut u8, length: usize) -> Status {
        unsafe {
            if Area::check_overlapping(to, length).is_err() {
                return Status::Invalid;
            }
            core::ptr::copy_nonoverlapping(
                EXCHANGE_AREA.as_ptr(),
                to,
                length.min(EXCHANGE_AREA_LEN),
            );
        }
        Status::Ok
    }
}


impl Area {

    fn new() -> Self {
        Self { }
    }

    #[allow(static_mut_refs)]
    unsafe fn check_overlapping(pointer: *const u8, length: usize) -> Result<(), ()> {
        let area = EXCHANGE_AREA.as_ptr();
        let area_end = area.add(EXCHANGE_AREA_LEN);

        // buffer starts in the middle of the exchange area, abort
        if pointer >= area && pointer <= area_end {
            return Err(());
        }

        // buffer ends in the exchange area, abort
        // Note: this is unlikely to happen if `svc_exchange` is always assumed to be at
        // the beginning of RAM
        let buffer_end = pointer.add(length);
        if buffer_end >= area && buffer_end <= area_end {
            return Err(());
        }

        // exchange area is contained within the buffer, abort
        if pointer <= area && buffer_end >= area_end {
            return Err(());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_area() {
        let area = Area::new();
        assert_eq!(<Area as ExhangeArea<u8>>::area_length(&area), 128);
    }

    #[test]
    fn back_to_back_copy() {
        let area = Area::new();
        let string = [b'z'; 100];
        let mut res = [b'a'; 100];
        area.copy_to(string.as_ptr(), string.len());
        area.copy_from(res.as_mut_ptr(), string.len());
        assert_eq!(res, string);
    }
}
