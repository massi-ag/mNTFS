use libmnfts::io::BlockReader;
use std::io::Cursor;

#[test]
fn test_block_reader_reads_at_offset() {
    let mut data = vec![0u8; 4096];
    data[512] = 0xAB;
    data[513] = 0xCD;

    let reader = BlockReader::from_reader(Cursor::new(data), 512);
    let block = reader.read_block(1).unwrap();
    assert_eq!(block[0], 0xAB);
    assert_eq!(block[1], 0xCD);
    assert_eq!(block.len(), 512);
}

#[test]
fn test_block_reader_out_of_bounds() {
    let data = vec![0u8; 1024];
    let reader = BlockReader::from_reader(Cursor::new(data), 512);
    // Only 2 blocks exist (0 and 1)
    assert!(reader.read_block(2).is_err());
}

#[test]
fn test_block_reader_read_bytes() {
    let mut data = vec![0u8; 4096];
    data[100] = 0xFF;
    data[101] = 0xFE;

    let reader = BlockReader::from_reader(Cursor::new(data), 512);
    let bytes = reader.read_bytes(100, 2).unwrap();
    assert_eq!(bytes, vec![0xFF, 0xFE]);
}

#[test]
fn test_block_reader_total_blocks() {
    let data = vec![0u8; 2048];
    let reader = BlockReader::from_reader(Cursor::new(data), 512);
    assert_eq!(reader.total_blocks(), 4);
}

use libmnfts::io::BlockCache;

#[test]
fn test_cache_stores_and_retrieves() {
    let mut cache = BlockCache::new(4);
    cache.put(0, vec![0xAA; 512]);
    cache.put(1, vec![0xBB; 512]);

    assert_eq!(cache.get(0).unwrap()[0], 0xAA);
    assert_eq!(cache.get(1).unwrap()[0], 0xBB);
    assert!(cache.get(2).is_none());
}

#[test]
fn test_cache_evicts_lru() {
    let mut cache = BlockCache::new(2);
    cache.put(0, vec![0xAA; 512]);
    cache.put(1, vec![0xBB; 512]);
    // Access block 0 to make it recently used
    cache.get(0);
    // Insert block 2, should evict block 1 (LRU)
    cache.put(2, vec![0xCC; 512]);

    assert!(cache.get(0).is_some());
    assert!(cache.get(1).is_none());
    assert!(cache.get(2).is_some());
}
