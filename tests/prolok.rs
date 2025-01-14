mod common;

use fluxfox::prelude::*;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_prolok() {
    use std::io::Cursor;
    init();

    let disk_image_buf = std::fs::read(".\\tests\\images\\monster_disk\\monster_disk_360k.pri").unwrap();
    let mut in_buffer = Cursor::new(disk_image_buf);
    let mut disk = DiskImage::load(&mut in_buffer, None, None, None).unwrap();

    println!("Loaded PRI image of geometry {}...", disk.image_format().geometry);

    let rsr = match disk.read_sector(
        DiskCh::new(39, 0),
        DiskChsnQuery::new(39, 0, 5, None),
        None,
        None,
        RwScope::DataOnly,
        false,
    ) {
        Ok(result) => result,
        Err(DiskImageError::DataError) => {
            panic!("Data error reading sector.");
        }
        Err(e) => panic!("Error reading sector: {:?}", e),
    };

    let sector_data = rsr.data();
    let original_data = sector_data.to_vec();

    println!(
        "Read sector data: {:02X?} of length {}",
        &sector_data[0..8],
        sector_data.len()
    );

    assert_eq!(sector_data.len(), 512);

    match disk.write_sector(
        DiskCh::new(39, 0),
        DiskChsnQuery::new(39, 0, 5, 2),
        None,
        sector_data,
        RwScope::DataOnly,
        false,
        false,
    ) {
        Ok(result) => result,
        Err(DiskImageError::DataError) => {
            panic!("Data error writing sector.");
        }
        Err(e) => panic!("Error writing sector: {:?}", e),
    };

    // Read the sector back. It should have different data.
    let rsr = match disk.read_sector(
        DiskCh::new(39, 0),
        DiskChsnQuery::new(39, 0, 5, 2),
        None,
        None,
        RwScope::DataOnly,
        false,
    ) {
        Ok(result) => result,
        Err(DiskImageError::DataError) => {
            panic!("Data error reading sector.");
        }
        Err(e) => panic!("Error reading sector: {:?}", e),
    };

    let sector_data = rsr.data();

    if sector_data.len() == 512 {
        println!("Original data: {:02X?}", &original_data[0..8]);
        println!("Post-write data: {:02X?}", &sector_data[0..8]);
    }

    if sector_data == original_data {
        panic!("Data read back from written sector did not change - no hole detected!");
    }

    println!("Data read back from written sector changed - hole detected!");
}
