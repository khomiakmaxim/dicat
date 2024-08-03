use clap::Parser;
use dicat::{prompt_parser::Args, App};

/// dicat restruct -p my_path --zip-only
/// dicat catalog -p my_path --as-csv

// DICAT ..
// catalog — виводимо стурктуровану інофрмацію про DICOM файли в ієрархії
// Можна вивести структурована і гарно. Ці таблички і їх обробку можна зробити через indicatiff

// Якщо не самі завантаження, то можна спробувати додати ці мигаючі рядочки

/// |---------------------------
/// |(ID xxxx) Maksym Khomiak:
/// |- - - - - - - - - - - - - - -
/// |123:
/// |    inner:
/// |        213.dcm
/// |        21.dcm
/// |        other_inner:
/// |            a.dicom
/// |            b.dicom
/// |    123:
/// |        321:
/// |            dir:
/// |                a.dcm
/// |                b.dcm
/// |        b.dcm
/// |    a.dcm
/// |321:
/// |    a.dcm
/// |    
/// |--------------------------------
/// |(ID yyyy) Oleh Tiagnybok:
/// |- - - - - - - - - - - - - - - -
/// |123:
/// |    inner:
/// |        213.dcm
/// |        21.dcm
/// |        other_inner:
/// |            a.dicom
/// |            b.dicom
/// |    123:
/// |        321:
/// |            dir:
/// |                a.dcm
/// |                b.dcm
/// |        b.dcm
/// |    a.dcm
/// |321:
/// |    a.dcm
/// |------------------------------
///

/// `restruct` необхідну папку, копіюючи всі необхідні файли + опціонально дасть можливість зробити якийсь архів
/// `--zip` // додасть ще й в zip
/// `--only-zip` // створить лише `zip`

// Може є випадки, коли було б добре зберегти стару структуру?
// Мабуть, що є

/// Тобто в нас мають бути 2 структури: Гарна й структурована і така, що впорядку обходу ієрархії
/// Останню можна навіть зробити проситм walkdir

/// Крмі цього я б ще хотів додати можливість зводити в CSV формат
///
/// Id xxxxx,Maksym Khomiak,123/321/a.dcm
/// Id xxxxx,Maksym Khomiak,123/321/a.dcm
/// Id xxxxx,Maksym Khomiak,123/321/a.dcm
/// Id xxxxx,Maksym Khomiak,123/321/a.dcm
/// Id xxxxx,Maksym Khomiak,123/321/a.dcm
/// Id yyyyy,Oleh Tiahnybok,321/321/b.dicom
///
///
fn main() {
    let args = Args::parse();

    // TODO: use anyhow|this_error
    if let Err(err) = App::start(args) {
        eprintln!("Something went wrong: {}", err)
    }
}
