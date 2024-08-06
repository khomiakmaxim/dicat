# Dicat
Command line utility that catalogs DICOM files

#Usage
## 1. Build the project
``
cargo b
``

## 2. Run tests
``
cargo t
``

## 3. Run with `--help` for useful information
``
target/debug/dicat --help
``
![image](https://github.com/user-attachments/assets/16c7e81a-3068-472f-b1b6-30033fc81b3f)
## 4. Try to run `catalog` command on a directory, which contains `DICOM` files
``
target/debug/dicat catalog --path 
``
![image](https://github.com/user-attachments/assets/80c241fd-4e91-4e1e-b880-2fae8145e455)
For each patient, which `DICOM` files were in the original folder a rectangle with original directory sub-tree will be printed to the console
