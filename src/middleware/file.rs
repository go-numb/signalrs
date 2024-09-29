use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use std::io::Result;

use serde::{Deserialize, Serialize};
use serde_json;

// create temp dir in app dir
pub fn create_save_dir(target_path: &Path) -> Result<()> {
    // exists or create
    if !std::path::Path::new(target_path).exists() {
        std::fs::create_dir(target_path)?;
    }

    Ok(())
}

pub fn write<T: Serialize>(filename: &str, setting: &T) -> Result<()> {
    let file = File::create(filename)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, setting)?;
    Ok(())
}

pub fn read<T>(filename: &str) -> Result<T>
where
    for<'de> T: Deserialize<'de>,
{
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Setting {
        value: String,
        label: String,
    }

    // create app save dir
    #[test]
    fn test_create_save_dir() {
        let path = "./.test/test.json";
        let dir = Path::new(path).parent().unwrap();
        create_save_dir(dir).unwrap();
        assert!(std::path::Path::new(dir).exists());
        fs::remove_dir(dir).unwrap();
    }

    // create "temp/test.json" file, write and read
    #[test]
    fn test_write_read() {
        let path = "./.test/test.json";
        let dir = Path::new(path).parent().unwrap();
        create_save_dir(dir).unwrap();
        let setting = Setting {
            value: "0".to_string(),
            label: "option".to_string(),
        };
        write(path, &setting).unwrap();
        let read_setting: Setting = read::<Setting>(path).unwrap();
        assert_eq!(setting, read_setting);
        fs::remove_dir_all(dir).unwrap();
    }
}
