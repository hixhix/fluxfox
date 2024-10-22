/*
    FluxFox
    https://github.com/dbalsom/fluxfox

    Copyright 2024 Daniel Balsom

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the “Software”),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    --------------------------------------------------------------------------

    src/track/metasector.rs

    Implements the MetaSector track type and the Track trait for same.

*/
use super::{Track, TrackConsistency, TrackInfo};

use crate::diskimage::{
    ReadSectorResult, ReadTrackResult, RwSectorScope, ScanSectorResult, SectorDescriptor, WriteSectorResult,
};

use crate::structure_parsers::system34::System34Standard;
use crate::structure_parsers::DiskStructureMetadata;

use crate::{DiskCh, DiskChs, DiskChsn, DiskDataEncoding, DiskDataRate, DiskImageError, FoxHashSet, SectorMapEntry};
use sha1_smol::Digest;
use std::any::Any;

struct SectorMatch<'a> {
    pub(crate) sectors: Vec<&'a MetaSector>,
    pub(crate) sizes: Vec<u8>,
    pub(crate) wrong_cylinder: bool,
    pub(crate) bad_cylinder: bool,
    pub(crate) wrong_head: bool,
}

impl SectorMatch<'_> {
    fn len(&'_ self) -> usize {
        self.sectors.len()
    }
    fn iter(&'_ self) -> std::slice::Iter<&MetaSector> {
        self.sectors.iter()
    }
}

struct SectorMatchMut<'a> {
    pub(crate) sectors: Vec<&'a mut MetaSector>,
    pub(crate) sizes: Vec<u8>,
    pub(crate) wrong_cylinder: bool,
    pub(crate) bad_cylinder: bool,
    pub(crate) wrong_head: bool,
}

impl<'a> SectorMatchMut<'a> {
    fn len(&'a self) -> usize {
        self.sectors.len()
    }
    fn iter_mut(&'a mut self) -> std::slice::IterMut<&mut MetaSector> {
        self.sectors.iter_mut()
    }
}

#[derive(Default)]
struct MetaMask {
    has_bits: bool,
    mask: Vec<u8>,
}

impl MetaMask {
    fn empty(len: usize) -> MetaMask {
        MetaMask {
            has_bits: false,
            mask: vec![0; len],
        }
    }
    fn from(mask: &[u8]) -> MetaMask {
        let mut m = MetaMask::default();
        m.set_mask(mask);
        m
    }
    fn set_mask(&mut self, mask: &[u8]) {
        self.mask = mask.to_vec();
        self.has_bits = mask.iter().any(|&x| x != 0);
    }
    #[allow(dead_code)]
    fn or_mask(&mut self, source_mask: &MetaMask) {
        for (i, &m) in source_mask.iter().enumerate() {
            self.mask[i] |= m;
        }
        self.has_bits = self.mask.iter().any(|&x| x != 0);
    }
    fn or_slice(&mut self, source_mask: &[u8]) {
        for (i, &m) in source_mask.iter().enumerate() {
            self.mask[i] |= m;
        }
        self.has_bits = self.mask.iter().any(|&x| x != 0);
    }
    fn clear(&mut self) {
        self.mask.fill(0);
        self.has_bits = false;
    }
    fn mask(&self) -> &[u8] {
        &self.mask
    }
    fn has_bits(&self) -> bool {
        self.has_bits
    }
    fn iter(&self) -> std::slice::Iter<u8> {
        self.mask.iter()
    }
    fn len(&self) -> usize {
        self.mask.len()
    }
}

pub(crate) struct MetaSector {
    id_chsn: DiskChsn,
    address_crc_error: bool,
    data_crc_error: bool,
    deleted_mark: bool,
    missing_data: bool,
    data: Vec<u8>,
    weak_mask: MetaMask,
    hole_mask: MetaMask,
}

impl MetaSector {
    pub fn read_data(&self) -> Vec<u8> {
        if self.missing_data {
            return Vec::new();
        }
        let mut data = self.data.clone();
        for (i, (weak_byte, hole_byte)) in self.weak_mask.iter().zip(self.hole_mask.iter()).enumerate() {
            let mask_byte = weak_byte | hole_byte;
            if mask_byte == 0 {
                continue;
            }
            let rand_byte = rand::random::<u8>();
            data[i] = data[i] & !mask_byte | rand_byte & mask_byte;
        }
        data
    }
}

pub struct MetaSectorTrack {
    pub(crate) ch: DiskCh,
    pub(crate) encoding: DiskDataEncoding,
    pub(crate) data_rate: DiskDataRate,
    pub(crate) sectors: Vec<MetaSector>,
}

impl Track for MetaSectorTrack {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn ch(&self) -> DiskCh {
        self.ch
    }

    fn set_ch(&mut self, new_ch: DiskCh) {
        self.ch = new_ch;
    }

    fn info(&self) -> TrackInfo {
        TrackInfo {
            encoding: self.encoding,
            data_rate: self.data_rate,
            bit_length: 0,
            sector_ct: self.sectors.len(),
        }
    }

    fn metadata(&self) -> Option<&DiskStructureMetadata> {
        None
    }

    fn get_sector_ct(&self) -> usize {
        self.sectors.len()
    }

    fn has_sector_id(&self, sid: u8, id_chsn: Option<DiskChsn>) -> bool {
        if let Some(_) = &self.sectors.iter().find(|sector| {
            if id_chsn.is_none() && sector.id_chsn.s() == sid {
                return true;
            } else if let Some(chsn) = id_chsn {
                if sector.id_chsn == chsn {
                    return true;
                }
            }
            false
        }) {
            true
        } else {
            false
        }
    }

    fn get_sector_list(&self) -> Vec<SectorMapEntry> {
        self.sectors
            .iter()
            .map(|s| SectorMapEntry {
                chsn: s.id_chsn,
                address_crc_valid: !s.address_crc_error,
                data_crc_valid: !s.data_crc_error,
                deleted_mark: s.deleted_mark,
                no_dam: false,
            })
            .collect()
    }

    fn add_sector(&mut self, sd: &SectorDescriptor, alternate: bool) -> Result<(), DiskImageError> {
        // Create an empty weak bit mask if none is provided.
        let weak_mask = match &sd.weak_mask {
            Some(weak_buf) => MetaMask::from(weak_buf),
            None => MetaMask::empty(sd.data.len()),
        };

        let hole_mask = match &sd.hole_mask {
            Some(hole_buf) => MetaMask::from(hole_buf),
            None => MetaMask::empty(sd.data.len()),
        };

        let new_sector = MetaSector {
            id_chsn: sd.id_chsn,
            address_crc_error: sd.address_crc_error,
            data_crc_error: sd.data_crc_error,
            deleted_mark: sd.deleted_mark,
            missing_data: sd.missing_data,
            data: sd.data.clone(),
            weak_mask,
            hole_mask,
        };

        if alternate {
            // Look for existing sector.
            let existing_sector = self.sectors.iter_mut().find(|s| s.id_chsn == sd.id_chsn);

            if let Some(es) = existing_sector {
                // Update the existing sector.
                let mut xor_vec: Vec<u8> = Vec::with_capacity(es.data.len());

                // Calculate a bitmap representing the difference between the new sector data and the
                // existing sector data.
                for (i, (ns_byte, es_byte)) in new_sector.data.iter().zip(es.data.iter()).enumerate() {
                    xor_vec[i] = ns_byte ^ es_byte;
                }

                // Update the weak bit mask for the existing sector and return.
                es.weak_mask.or_slice(&xor_vec);
                return Ok(());
            }
        }

        self.sectors.push(new_sector);

        Ok(())
    }

    /// Read the sector data from the sector identified by 'chs'. The data is returned within a
    /// ReadSectorResult struct which also sets some convenience metadata flags where are needed
    /// when handling ByteStream images.
    /// When reading a BitStream image, the sector data includes the address mark and crc.
    /// Offsets are provided within ReadSectorResult so these can be skipped when processing the
    /// read operation.
    fn read_sector(
        &mut self,
        chs: DiskChs,
        n: Option<u8>,
        scope: RwSectorScope,
        debug: bool,
    ) -> Result<ReadSectorResult, DiskImageError> {
        match scope {
            // Add 4 bytes for address mark and 2 bytes for CRC.
            RwSectorScope::DataBlock => unimplemented!("DataBlock scope not supported for ByteStream"),
            RwSectorScope::DataOnly => {}
        };

        let sm = self.match_sectors(chs, n, debug);

        if sm.len() == 0 {
            log::debug!("read_sector(): No sector found for chs: {} n: {:?}", chs, n);
            Ok(ReadSectorResult {
                id_chsn: None,
                data_idx: 0,
                data_len: 0,
                read_buf: Vec::new(),
                deleted_mark: false,
                not_found: true,
                no_dam: false,
                address_crc_error: false,
                data_crc_error: false,
                wrong_cylinder: sm.wrong_cylinder,
                bad_cylinder: sm.bad_cylinder,
                wrong_head: sm.wrong_head,
            })
        } else {
            if sm.len() > 1 {
                log::warn!(
                    "read_sector(): Found {} sector ids matching chs: {} (with {} different sizes). Using first.",
                    sm.len(),
                    chs,
                    sm.sizes.len()
                );
            }
            let s = sm.sectors[0];

            Ok(ReadSectorResult {
                id_chsn: Some(s.id_chsn),
                data_idx: 0,
                data_len: s.data.len(),
                read_buf: s.read_data(), // Calling read_data applies the weak bit and hole masks.
                deleted_mark: s.deleted_mark,
                not_found: false,
                no_dam: false,
                address_crc_error: s.address_crc_error,
                data_crc_error: s.data_crc_error,
                wrong_cylinder: sm.wrong_cylinder,
                bad_cylinder: sm.bad_cylinder,
                wrong_head: sm.wrong_head,
            })
        }
    }

    fn scan_sector(&self, chs: DiskChs, n: Option<u8>) -> Result<ScanSectorResult, DiskImageError> {
        let sm = self.match_sectors(chs, n, false);

        if sm.len() == 0 {
            log::debug!("scan_sector(): No sector found for chs: {} n: {:?}", chs, n);
            Ok(ScanSectorResult {
                not_found: true,
                no_dam: false,
                deleted_mark: false,
                address_crc_error: false,
                data_crc_error: false,
                wrong_cylinder: sm.wrong_cylinder,
                bad_cylinder: sm.bad_cylinder,
                wrong_head: sm.wrong_head,
            })
        } else {
            log::warn!(
                "read_sector(): Found {} sector ids matching chs: {} (with {} different sizes). Using first.",
                sm.len(),
                chs,
                sm.sizes.len()
            );
            let s = sm.sectors[0];

            Ok(ScanSectorResult {
                deleted_mark: s.deleted_mark,
                not_found: false,
                no_dam: false,
                address_crc_error: s.address_crc_error,
                data_crc_error: s.data_crc_error,
                wrong_cylinder: sm.wrong_cylinder,
                bad_cylinder: sm.bad_cylinder,
                wrong_head: sm.wrong_head,
            })
        }
    }

    fn write_sector(
        &mut self,
        chs: DiskChs,
        n: Option<u8>,
        write_data: &[u8],
        _scope: RwSectorScope,
        write_deleted: bool,
        debug: bool,
    ) -> Result<WriteSectorResult, DiskImageError> {
        let mut sm = self.match_sectors_mut(chs, n, debug);

        if sm.len() > 1 {
            log::error!(
                "write_sector(): Could not identify unique target sector. (Found {} sector ids matching chs: {} n: {:?})",
                sm.len(),
                chs,
                n
            );
            return Err(DiskImageError::UniqueIdError);
        } else if sm.len() == 0 {
            log::debug!("write_sector(): No sector found for chs: {} n: {:?}", chs, n);
            return Ok(WriteSectorResult {
                not_found: false,
                no_dam: false,
                address_crc_error: false,
                wrong_cylinder: sm.wrong_cylinder,
                bad_cylinder: sm.bad_cylinder,
                wrong_head: sm.wrong_head,
            });
        }

        let write_data_len = write_data.len();
        if DiskChsn::n_to_bytes(sm.sectors[0].id_chsn.n()) != write_data_len {
            // Caller didn't provide correct buffer size.
            log::error!(
                "write_sector(): Data buffer size mismatch, expected: {} got: {}",
                DiskChsn::n_to_bytes(sm.sectors[0].id_chsn.n()),
                write_data_len
            );
            return Err(DiskImageError::ParameterError);
        }

        if sm.sectors[0].missing_data || sm.sectors[0].address_crc_error {
            log::debug!(
                "write_sector(): Sector {} is unwritable due to no DAM or bad address CRC.",
                sm.sectors[0].id_chsn
            );
        } else {
            sm.sectors[0].data.copy_from_slice(write_data);
            sm.sectors[0].deleted_mark = write_deleted;
        }

        Ok(WriteSectorResult {
            not_found: false,
            no_dam: sm.sectors[0].missing_data,
            address_crc_error: sm.sectors[0].address_crc_error,
            wrong_cylinder: sm.wrong_cylinder,
            bad_cylinder: sm.bad_cylinder,
            wrong_head: sm.wrong_head,
        })
    }

    fn get_hash(&mut self) -> Digest {
        let mut hasher = sha1_smol::Sha1::new();
        let rtr = self.read_all_sectors(self.ch, 0xFF, 0xFF).unwrap();
        hasher.update(&rtr.read_buf);
        hasher.digest()
    }

    /// Read all sectors from the track identified by 'ch'. The data is returned within a
    /// ReadSectorResult struct which also sets some convenience metadata flags which are needed
    /// when handling ByteStream images.
    /// Unlike read_sectors, the data returned is only the actual sector data. The address marks and
    /// CRCs are not included in the data.
    /// This function is intended for use in implementing the Read Track FDC command.
    fn read_all_sectors(&mut self, _ch: DiskCh, n: u8, track_len: u8) -> Result<ReadTrackResult, DiskImageError> {
        let track_len = track_len as u16;
        let sector_data_len = DiskChsn::n_to_bytes(n);
        let mut track_read_vec = Vec::with_capacity(sector_data_len * self.sectors.len());
        let mut address_crc_error = false;
        let mut data_crc_error = false;
        let mut deleted_mark = false;

        let mut not_found = true;
        let mut sectors_read = 0;

        for s in &self.sectors {
            log::trace!("read_all_sectors(): Found sector_id: {}", s.id_chsn,);
            not_found = false;

            // TODO - do we stop after reading sector ID specified by EOT, or
            //        or upon reaching it?
            if sectors_read >= track_len {
                log::trace!(
                    "read_all_sectors(): Reached track_len at sector: {} \
                        sectors_read: {}, track_len: {}",
                    s.id_chsn,
                    sectors_read,
                    track_len
                );
                break;
            }

            track_read_vec.extend(&s.read_data());
            sectors_read = sectors_read.saturating_add(1);

            if s.address_crc_error {
                address_crc_error |= true;
            }

            if s.data_crc_error {
                data_crc_error |= true;
            }

            if s.deleted_mark {
                deleted_mark |= true;
            }
        }
        let read_len = track_read_vec.len();
        Ok(ReadTrackResult {
            not_found,
            sectors_read,
            read_buf: track_read_vec,
            deleted_mark,
            address_crc_error,
            data_crc_error,
            read_len_bits: read_len * 16,
            read_len_bytes: read_len,
        })
    }

    fn get_next_id(&self, chs: DiskChs) -> Option<DiskChsn> {
        let first_sector = self.sectors.first()?;
        let mut sector_matched = false;
        for si in self.sectors.iter() {
            if sector_matched {
                return Some(DiskChsn::new(chs.c(), chs.h(), si.id_chsn.s(), si.id_chsn.n()));
            }
            if si.id_chsn.s() == chs.s() {
                // Have matching sector id
                sector_matched = true;
            }
        }
        // If we reached here, we matched the last sector in the list, so return the first
        // sector as we wrap around the track.
        if sector_matched {
            Some(DiskChsn::new(
                chs.c(),
                chs.h(),
                first_sector.id_chsn.s(),
                first_sector.id_chsn.n(),
            ))
        } else {
            None
        }
    }

    fn read_track(&mut self, _overdump: Option<usize>) -> Result<ReadTrackResult, DiskImageError> {
        Err(DiskImageError::UnsupportedFormat)
    }

    fn has_weak_bits(&self) -> bool {
        self.sectors.iter().map(|s| s.weak_mask.has_bits()).any(|x| x)
    }

    fn format(
        &mut self,
        _standard: System34Standard,
        _format_buffer: Vec<DiskChsn>,
        _fill_pattern: &[u8],
        _gap3: usize,
    ) -> Result<(), DiskImageError> {
        // TODO: Implement format for MetaSectorTrack
        Err(DiskImageError::UnsupportedFormat)
    }

    fn get_track_consistency(&self) -> TrackConsistency {
        let sector_ct;

        let mut consistency = TrackConsistency::default();

        sector_ct = self.sectors.len();

        let mut n_set: FoxHashSet<u8> = FoxHashSet::new();
        let mut last_n = 0;
        for (si, sector) in self.sectors.iter().enumerate() {
            if sector.id_chsn.s() != si as u8 + 1 {
                consistency.nonconsecutive_sectors = true;
            }
            if sector.data_crc_error {
                consistency.bad_data_crc = true;
            }
            if sector.address_crc_error {
                consistency.bad_address_crc = true;
            }
            if sector.deleted_mark {
                consistency.deleted_data = true;
            }
            last_n = sector.id_chsn.n();
            n_set.insert(sector.id_chsn.n());
        }

        if n_set.len() > 1 {
            consistency.consistent_sector_size = None;
        } else {
            consistency.consistent_sector_size = Some(last_n);
        }

        consistency.sector_ct = sector_ct;
        consistency
    }
}

impl MetaSectorTrack {
    fn match_sectors(&self, chs: DiskChs, n: Option<u8>, debug: bool) -> SectorMatch {
        let mut wrong_cylinder = false;
        let mut bad_cylinder = false;
        let mut wrong_head = false;

        let mut sizes = FoxHashSet::new();
        let matching_sectors: Vec<&MetaSector> = self
            .sectors
            .iter()
            .filter(|s| {
                let matched_s = s.id_chsn.s() == chs.s();
                if s.id_chsn.c() != chs.c() {
                    wrong_cylinder = true;
                }
                if s.id_chsn.c() == 0xFF {
                    bad_cylinder = true;
                }
                if s.id_chsn.h() != chs.h() {
                    wrong_head = true;
                }
                sizes.insert(s.id_chsn.n());
                (debug && matched_s)
                    || ((DiskChs::from(s.id_chsn) == chs) && (n.is_none() || s.id_chsn.n() == n.unwrap()))
            })
            .collect();

        SectorMatch {
            sectors: matching_sectors,
            sizes: sizes.iter().cloned().collect(),
            wrong_cylinder,
            bad_cylinder,
            wrong_head,
        }
    }

    fn match_sectors_mut(&mut self, chs: DiskChs, n: Option<u8>, debug: bool) -> SectorMatchMut {
        let mut wrong_cylinder = false;
        let mut bad_cylinder = false;
        let mut wrong_head = false;

        let mut sizes = FoxHashSet::new();
        let matching_sectors: Vec<&mut MetaSector> = self
            .sectors
            .iter_mut()
            .filter(|s| {
                let matched_s = s.id_chsn.s() == chs.s();
                if s.id_chsn.c() != chs.c() {
                    wrong_cylinder = true;
                }
                if s.id_chsn.c() == 0xFF {
                    bad_cylinder = true;
                }
                if s.id_chsn.h() != chs.h() {
                    wrong_head = true;
                }
                sizes.insert(s.id_chsn.n());
                (debug && matched_s)
                    || ((DiskChs::from(s.id_chsn) == chs) && (n.is_none() || s.id_chsn.n() == n.unwrap()))
            })
            .collect();

        SectorMatchMut {
            sectors: matching_sectors,
            sizes: sizes.iter().cloned().collect(),
            wrong_cylinder,
            bad_cylinder,
            wrong_head,
        }
    }
}