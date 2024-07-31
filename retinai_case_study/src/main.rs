use dicom::{dictionary_std::tags, object::open_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let obj = open_file("test_dicom_files/0002.DCM")?;

    let patient_name = obj.element(tags::PATIENT_NAME)?.to_str()?;
    let patient_clinic = obj.element(tags::INSTITUTION_NAME)?.to_str()?;
    let patient_id = obj.element(tags::PATIENT_ID)?.to_str()?;

    dbg!(patient_name);
    dbg!(patient_clinic);
    dbg!(patient_id);

    Ok(())
}
