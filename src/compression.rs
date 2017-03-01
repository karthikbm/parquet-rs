// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::io;
use std::io::{Read, Write};

use basic::Compression as CodecType;
use errors::{Result, ParquetError};
use snap::{Decoder, Encoder, decompress_len};
use brotli;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

pub trait Codec {
  /// Compress data stored in slice `input_buf` and return a new
  /// vector with the compressed data.
  /// TODO: it's better to pass in vec here (e.g., allow reuse),
  ///   but flate2 api doesn't support this.
  fn compress(&mut self, input_buf: &[u8]) -> Result<Vec<u8>>;

  /// Decompress data stored in slice `input_buf` and write output
  /// to `output_buf`. Return the total number of bytes written.
  fn decompress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize>;
}


pub fn create_codec(codec: CodecType) -> Result<Box<Codec>> {
  match codec {
    CodecType::BROTLI => Ok(Box::new(BrotliCodec::new())),
    CodecType::GZIP => Ok(Box::new(GZipCodec::new())),
    CodecType::SNAPPY => Ok(Box::new(SnappyCodec::new())),
    _ => Err(unsupported_err!("The codec type {} is not supported yet", codec))
  }
}

pub struct SnappyCodec {
  decoder: Decoder,
  encoder: Encoder
}

impl SnappyCodec {
  fn new() -> Self {
    Self { decoder: Decoder::new(), encoder: Encoder::new() }
  }
}

impl Codec for SnappyCodec {
  fn decompress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize> {
    let len = decompress_len(input_buf)?;
    output_buf.resize(len, 0);
    self.decoder.decompress(input_buf, output_buf)
      .map_err(|e| io_err!(io::Error::from(e), "Error when decompressing using Snappy"))
  }

  fn compress(&mut self, input_buf: &[u8]) -> Result<Vec<u8>> {
    self.encoder.compress_vec(input_buf)
      .map_err(|e| io_err!(io::Error::from(e), "Error when compressing using Snappy"))
  }
}

pub struct GZipCodec {
}

impl GZipCodec {
  fn new() -> Self {
    Self { }
  }
}

impl Codec for GZipCodec {
  fn decompress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize> {
    let mut decoder = GzDecoder::new(input_buf)?;
    decoder.read_to_end(output_buf).map_err(|e| io_err!(e, "Error when decompressing using GZip"))
  }

  fn compress(&mut self, input_buf: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::Default);
    encoder.write_all(input_buf)?;
    encoder.finish().map_err(|e| io_err!(e, "Error when compressing using GZip"))
  }
}

pub struct BrotliCodec {
}

impl BrotliCodec {
  fn new() -> Self {
    Self { }
  }
}

impl Codec for BrotliCodec {
  fn decompress(&mut self, input_buf: &[u8], output_buf: &mut Vec<u8>) -> Result<usize>{
    brotli::Decompressor::new(input_buf, 4096).read(output_buf)
      .map_err(|e| io_err!(io::Error::from(e), "Error when decompressing using Brotli"))
  }

  fn compress(&mut self, _: &[u8]) -> Result<Vec<u8>> {
    // TODO: add this once rust-brotli implements the functionality
    unimplemented!()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use util::test_common::*;

  fn test_roundtrip(c: CodecType, data: &Vec<u8>) {
    let mut c1 = create_codec(c).unwrap();
    let mut c2 = create_codec(c).unwrap();

    // compress with c1
    let mut decompressed = Vec::new();
    let mut compressed_res = c1.compress(data.as_slice());
    assert!(compressed_res.is_ok());
    let mut compressed = compressed_res.unwrap();

    // decompress with c2
    let mut decompressed_size = c2.decompress(compressed.as_slice(), &mut decompressed);
    assert!(decompressed_size.is_ok());
    decompressed.truncate(decompressed_size.unwrap());
    assert!(*data == decompressed);

    // compress with c2
    compressed_res = c2.compress(data.as_slice());
    assert!(compressed_res.is_ok());
    compressed = compressed_res.unwrap();

    // decompress with c1
    decompressed_size = c1.decompress(compressed.as_slice(), &mut decompressed);
    assert!(decompressed_size.is_ok());
    decompressed.truncate(decompressed_size.unwrap());
    assert!(*data == decompressed);
  }

  fn test_codec(c: CodecType) {
    let sizes = vec![100, 10000, 100000];
    for size in sizes {
      let mut data = Vec::new();
      random_bytes(size, &mut data);
      test_roundtrip(c, &mut data);
    }
  }

  #[test]
  fn test_codec_snappy() {
    test_codec(CodecType::SNAPPY);
  }

  #[test]
  fn test_codec_gzip() {
    test_codec(CodecType::GZIP);
  }
}