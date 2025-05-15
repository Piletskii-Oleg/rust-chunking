[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/cdc-chunkers.svg
[crates-url]: https://crates.io/crates/cdc-chunkers
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/Piletskii-Oleg/rust-chunking/blob/main/LICENSE

# rust-chunking
Content Based Chunking algorithms implementation:
* RabinCDC (taken from [zbox][zbox])
* [Leap-based CDC][leap]
  * Matrix generation code can be found in [ef_matrix.rs](src/bin/ef_matrix.rs)
* [UltraCDC][ultra]
* [SuperCDC][super]
* [SeqCDC][seq]
* [Asymmetric Extremum][ae]
* [Rapid Asymmetric Maximum][ram]
* 
Simple code to test an algorithm is provided in [filetest.rs](src/bin/filetest.rs).

## Features

* Chunkers that work using `std::iter::Iterator` trait, giving out data about the source dataset 
in the form of chunks.
* Chunker sizes can be customized on creation. Default size values are provided.
* Other parameters from corresponding papers can also be modified on chunker creation.

## Usage

To use them in custom code, the algorithms can be accessed using the corresponding modules,
e.g. 
```rust
fn main() {
    let data = vec![1; 1024 * 1024];
    
    let sizes = SizeParams::new(4096, 8192, 16384);
    let chunker = ultra::Chunker::new(&data, sizes);
  
    for chunk in chunker {
        println!("start: {}, length: {}", chunk.pos, chunk.len);
    }
  
    let default_leap = leap_based::Chunker::new(&data, SizeParams::leap_default());
    for chunk in default_leap {
        println!("start: {}, length: {}", chunk.pos, chunk.len);
    }
}
```

[ultra]: https://ieeexplore.ieee.org/document/9894295/
[leap]: https://ieeexplore.ieee.org/document/7208290
[seq]: https://dl.acm.org/doi/10.1145/3652892.3700766
[super]: https://www.researchgate.net/publication/366434502_SuperCDC_A_Hybrid_Design_of_High-Performance_Content-Defined_Chunking_for_Fast_Deduplication
[zbox]: https://github.com/zboxfs/zbox
[ae]: https://ieeexplore.ieee.org/abstract/document/7524782/
[ram]: https://www.sciencedirect.com/science/article/pii/S0167739X16305829