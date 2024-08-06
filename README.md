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

## 5. You can choose a subset of patients via providing `--ids` option and listing patient's IDs separated by `,`

``
target/debug/dicat catalog --path --ids ID1,ID2
``
![image](https://github.com/user-attachments/assets/70d5fe58-679a-4e6f-9d69-a1de9143c146)

## 6. You also have an option to print information about the `DICOM` files in the directory in the `.csv` format, which preserves the original hierarchy of the directory
``
target/debug/dicat catalog --path --as-csv
``
![image](https://github.com/user-attachments/assets/b37b1617-8c53-4fc3-b877-1b89374610ab)

## 7. You can also restructure the `DICOM` files from the directory into a new one, which will contain separate directories for each patient with their `DICOM` files directly in them
``
target/debug/dicat restruct --path
``
![image](https://github.com/user-attachments/assets/dba6c817-d73b-4b56-81c3-a6332eb689b1)
You can check the structure of the newely created directory via the `catalog` command
![image](https://github.com/user-attachments/assets/f695666a-6156-4048-8335-21870e606422)

# Design issues
* At this point, there's no possibility to provide a path to the directory where you want to `restruct` your file

# Codebase issues
* It would be better to decouple parts, which scaffold the `catalog` structure, and which print it to the stdout by introducing a trait similar to `WriteTree`. Currently, that would require a codebase to be refactored
* The amount of `tokio` tasks which copy files into the newely created directory when using `restruct` is currenlty hardcoded to be `4`. It's the smallest amount of async I\O tasks, which use the maximum throughput of my SSD. 
  It would be better to either dynamically deduce this number, or, at least, provide a possibility to overwrite it via the argument or tne environment variable

# Dependency notes:
* For directory traversal I use `walkdir` for sequential and `jwalk` for parallel cases. Since both of them aren't widely known and are currently only being supported, I'd consider to fork them and work with the forked versions, in order to avoid possible issues in the future
