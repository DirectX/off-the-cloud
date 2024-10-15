use std::{cell::Cell, fs::File, io::{BufWriter, Write}, path::PathBuf};
use anyhow::Context;
use melib::mbox::{MboxFormat, MboxMetadata};

pub struct MboxWriter {
    pub writer: Cell<BufWriter<File>>,
    pub max_file_size: u64,
}

impl MboxWriter {
    pub fn new(file_path: PathBuf, max_file_size: u64) -> anyhow::Result<Self> {
        let file = File::create(file_path).context("file creation error")?;
        let writer = std::io::BufWriter::new(file);
        
        Ok(Self {
            writer: Cell::new(writer),
            max_file_size
        })
    }

    pub fn append(&self, _data: &[u8]) -> anyhow::Result<()> {
        // println!("adding message: {:?}", data);
        // let writer = self.writer.get_mut();

        // let data: &[u8] = br#"From: <a@b.c>\n\nHello World"#;
        // let format = MboxFormat::MboxCl2;
        // format.append(
        //     writer,
        //     data,
        //     None,
        //     Some(melib::utils::datetime::now()),
        //     Default::default(),
        //     MboxMetadata::None,
        //     true,
        //     false,
        // )?;

        Ok(())
    }

    pub fn flush(&self) -> anyhow::Result<()> {
        // anyhow::Result::from(self.file.get_mut().flush())
        Ok(())
    }
}