use anyhow::{bail, Result};
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
struct Cell {
    offset: u16
}
impl Cell {
    fn new(offset: u16) -> Self {
        Cell { offset }
    }
}
fn get_db_info(buffer: &Vec<u8>, page_size: u16, print_result: bool) -> DbInfo {
    let page_header_byte = buffer[100];
    let page_header_size = match page_header_byte {
        13 => 8,
        _ => 12,
    };
    let page_header = &buffer[100..100 + page_header_size];
    let tbl_count = u16::from_be_bytes([page_header[3], page_header[ 4]]);
    let db_info = DbInfo { no_tables: tbl_count as usize, db_page_size: page_size as usize, page_header_size, records: Records::new()};
    
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    if print_result {
        println!("database page size: {}", db_info.db_page_size);
        println!("number of tables: {}", db_info.no_tables);
    }
    return db_info
}

fn get_db_tables<'a> (db_info: &'a mut DbInfo, buffer: &Vec<u8>, print_result: bool) -> &'a Records {
    // Read master table 
    let mut cells: Vec<Cell> = Vec::new();
    let mut i = db_info.page_header_size + 100;
    loop {
        if buffer[i] == 0 && buffer[i+1] == 0 {
            break
        }
        cells.push(Cell::new(u16::from_be_bytes([buffer[i], buffer[i+1]])));
        i += 2
    }
    // Parsing records
    for cell in cells.iter() {
        let offset = usize::from(cell.offset);
        let (payload_size, payload_size_len) = decode_varint(&buffer[offset..]);
        let (rowid, rowid_len) = decode_varint(&buffer[offset + payload_size_len..]);
        let record_start = offset + payload_size_len + rowid_len;
        let (header_size, header_len) = decode_varint(&buffer[record_start..]);
        let payload_header = &buffer[record_start + header_len..record_start + header_size as usize];
        let rec_header = RecordHeader::new(payload_header, payload_size as usize, rowid as usize, header_size as usize);
        let rec_payload_start = record_start + rec_header.header_size;
        let record_payload = &buffer[rec_payload_start..rec_payload_start + rec_header.size - rec_header.header_size];
        let record = Record::new(record_payload, rec_header);
        if print_result {
        println!("{:?}\n", record);
        print!("{} ", record.tbl_name);
        }
        db_info.records.add_record(record);
}
        &db_info.records
}

fn main() -> Result<()> {
    // Parse arguments
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    // Parse command and act accordingly
    let command = &args[2];
    let mut file = File::open(&args[1])?;
    let mut header = [0; 100];
    file.read_exact(&mut header)?;
    #[allow(unused_variables)]
    let page_size = u16::from_be_bytes([header[16], header[17]]);
    let mut buffer = Vec::new();
    buffer.resize(page_size as usize, 0u8);
    file.read_exact(&mut buffer[100..])?;
    let mut db_info = get_db_info(&buffer, page_size, false);

    match command.as_str() {
        ".dbinfo" => {
            // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
           get_db_info(&buffer, page_size, true);
        },
        ".tables" => {
            // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
            get_db_tables(&mut db_info, &buffer, true);
        },
        statement => {
            let stms: Vec<&str> = statement.split(' ').collect(); 
            let stmt_tbl_name: String = match stms.last() {
                Some(word) => {
                    word.to_string()
                },
                None => {
                    panic!("Please enter a valid table name");
                }
            };
            let tbl_info = get_db_tables(&mut db_info, &buffer, false);
            if !tbl_info.contains(stmt_tbl_name.clone()){
                println!("table: {} doesn't exist", &stmt_tbl_name);
                return Ok(());
            };
            println!("table: {} exists in the db", stmt_tbl_name);
        },
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

#[derive(Debug)]
struct RecordHeader {
    size: usize,
    rowid: usize,
    header_size: usize,
    type_size: usize,
    name_size: usize,
    tbl_name_size: usize,
    root_page: usize,
    sql_size: usize,
}

impl RecordHeader {
    fn new(buf: &[u8], payload_size: usize, rowid: usize, header_size: usize) -> Self {
        let mut cursor = 0;
        let mut serials = Vec::new();
        while cursor < buf.len() as usize {
            let (serial, slen) = decode_varint(&buf[cursor..]);
            serials.push(serial);
            cursor += slen;
        }

        let type_size = ((serials[0] - 13) / 2) as usize;
        let name_size = ((serials[1] - 13) / 2) as usize;
        let tbl_name_size = ((serials[2] - 13) / 2) as usize;
        let root_page = serials[3] as usize;
        let sql_size = ((serials[4] - 13) / 2) as usize;

        RecordHeader {
            size: payload_size,
            rowid, 
            header_size,
            type_size,
            name_size,
            tbl_name_size,
            root_page,
            sql_size,
        }
    }
}
fn convert_from_ascii(arr: &[u8]) -> String {
    let mut res = String::new();
    for i in arr.iter() {
        res.push(i.clone() as char);
    }
    res
}
fn decode_varint(buf: &[u8]) -> (u64, usize) {
    let mut value: u64 = 0;
    let mut consumed = 0;

    for &b in buf.iter().take(9) {
        consumed += 1;

        if b < 0x80 {
            // last byte: full 8 bits
            value = (value << 7) | (b as u64);
            break;
        } else {
            // continuation byte: lower 7 bits only
            value = (value << 7) | ((b & 0x7F) as u64);
        }
    }

    (value, consumed)
}

#[derive(Debug)]
struct Record {
    s_type: String,
    name: String,
    tbl_name: String,
    sql: String,
    header: RecordHeader,
}

impl Record {
    fn new(record_payload: &[u8], record_header: RecordHeader) -> Self {
        let mut i = 0;
        let s_type = convert_from_ascii(&record_payload[i..record_header.type_size + i]);
        i = record_header.type_size + i;
        let name = convert_from_ascii(&record_payload[i..record_header.name_size + i]);
        i = record_header.name_size + i;
        let tbl_name = convert_from_ascii(&record_payload[i..record_header.tbl_name_size + i]);
        i = record_header.tbl_name_size + i;
        i = record_header.root_page + i;
        let sql = convert_from_ascii(&record_payload[i..record_header.sql_size + i]);
        Record { s_type, name, tbl_name, sql, header: record_header }
    }
}

#[derive(Debug)]
struct Records {
    records: Vec<Record>
}

impl Records {
    fn new() -> Self {
        Records { records: Vec::new() }
    }
    fn add_record(self: &mut Self, record: Record) {
        self.records.push(record);
    }
    fn contains(self: &Self, tbl_name: String) -> bool {
        for rec in self.records.iter() {
            if rec.tbl_name == tbl_name {
                return true;
            }
        }
        return false;
    }
}

struct DbInfo {
    no_tables: usize,
    db_page_size: usize,
    page_header_size: usize,
    records: Records,
}
