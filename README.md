# rustqoi - simple QOI encoder/decoder  
<a href="https://oql.avris.it/license/v1.2" target="_blank" rel="noopener"><img src="https://badgers.space/badge/License/OQL/pink" alt="License: OQL" style="vertical-align: middle;"/></a>

## build  

ensure you have rust installed. if not, install it via [rustup](https://rustup.rs/).  

clone the repository and build the project:  

```sh
git clone https://github.com/musicalskele/rustqoi
cd rustqoi  
cargo build --release  
```

the compiled binary will be in `target/release/rustqoi`.

## usage  

```sh
rustqoi <input> [output] [-v]
```

### PNG to QOI  
```sh
rustqoi image.png
```
this will generate `image.qoi`.  

### QOI to PNG  
```sh
rustqoi image.qoi
```
this will generate `image.png`.  

### specifying an output file  
```sh
rustqoi meow.png woof.qoi
```

### verbose mode  
```sh
rustqoi image.png -v  
```
## notes
- the tool only supports `.png` and `.qoi` files.  
- if the output file exists, you will be prompted before overwriting.  
- if an invalid file format is provided, the program will exit with an error.  

## license  
this project is licensed under the **[OQL license](https://oql.avris.it/license/v1.2)**.  
