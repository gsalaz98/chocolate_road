use std::env;
use std::fs::{read_dir, remove_file, File};
use std::io::{Error, ErrorKind, Read, Write};

use rusoto_s3;
use rusoto_s3::{S3, S3Client};
use xz2::read::XzEncoder;

/// Compresses the DTF database, with the path loaded from environment variable `DTF_DB_PATH`
/// Optionally, a path can be supplied to the function as an Optional parameter.
///
/// Database is first stored as a tar file, and is then xz compressed at the highest level possible.
/// The `tar.xz` file will then be ready to be moved into Amazon S3 for long term storage
///
/// We require the `db_name` parameter because it's easier to maintain and encode information about
/// the file externally than it is to figure it out inside the function.
///
/// # Parameters
/// `db_name`: Filename the final tar.xz archive will have
/// `db_path`: Path to location of the DTF database (on disk)
pub fn compress_database_and_delete(db_name: &String, db_path: Option<String>) -> Result<(), Error> {
    // Define the database path location. We will try and match against environment variables before
    // falling back into a hardcoded default path. TODO: avoid using hardcoded path
    let db_path = db_path.unwrap_or(match env::var("DTF_DB_PATH") {
        Ok(db) => db,
        Err(_) => env::var("HOME").unwrap() + "/tectonicdb/target/release/db"
    });

    let mut db_tar = File::create(db_name)?;
    let mut db_tar_builder = tar::Builder::new(&db_tar);

    // Add all files inside the dtf database folder and name the folder "db"
    db_tar_builder.append_dir_all("db", &db_path)?;
    // Create and write the tar archive
    db_tar_builder.into_inner()?;

    // Create XzEncoder instance with new file open to avoid 'Bad file descriptor' error.
    let mut xz_enc = XzEncoder::new(File::open(db_name)?, 9);
    let mut xz_buf = vec![];

    // Read compressed xz bytes to a buffer
    xz_enc.read_to_end(&mut xz_buf)?;

    // Close XZ compressor
    drop(xz_enc);

    // Reopen db tar file to write compressed bytes
    //let mut db_tar = File::open(db_name).expect("Failed to open tar file for compression");

    // Finally, write compressed xz bytes to a file, overriding the tar contents
    db_tar.write_all(&mut xz_buf)?;

    // Delete all files in the tectonic database
    for dtf_file in read_dir(&db_path)? {
        let _ = remove_file(dtf_file?.path()).expect("Failed to delete DTF file");
    }

    Ok(())
}

/// Upload database (xz compressed) to the Amazon S3 bucket [`bucket`]. If [`bucket`] is `None`, then we will read the bucket
/// from the environment variable `S3_BUCKET`. We will default to `CuteQ` if we receive a `None`
/// value, and the environment variable is missing.
///
/// # Parameters
/// `db_name`: filename of the database tar file
/// `bucket`: S3 Bucket name we will upload to. Defaults to `cuteq`
/// `region`: Amazon AWS Region to use. Defaults to `us-east-1`
///
/// Issue: Does not upload to S3.
pub fn s3_upload(db_name: &String,
                 bucket: Option<String>,
                 region: Option<rusoto_core::Region>) -> Result<(), Error> {

    let credentials = rusoto_core::credential::ChainProvider::new();

    // Default to region us-east-1
    let region = region.unwrap_or(rusoto_core::Region::UsEast1);

    let s3 = S3Client::new_with(
        rusoto_core::request::HttpClient::new().unwrap(), 
        credentials, 
        region);

    // TODO: Remove hardcoded `cuteq` variable and load from Cargo.toml
    let bucket = bucket.unwrap_or(match env::var("S3_BUCKET") {
        Ok(bucket) => bucket,
        Err(_) => "cuteq".into()
    });

    let mut dtf_file = File::open(db_name)?;
    let mut dtf_buf = vec![];

    dtf_file.read_to_end(&mut dtf_buf)?;

    let s3_req = rusoto_s3::PutObjectRequest {
        bucket,
        body: Some(dtf_buf.into()),
        key: db_name.clone(),

        ..Default::default()
    };

    match s3.put_object(s3_req).sync() {
        Ok(msg) => {
            // TODO: implement logging
            println!("{:?}", msg);

            drop(dtf_file);
            //remove_file(db_name)?;

            return Ok(())
        },
        Err(e) => {
            // TODO: implement logging
            println!("{:?}", e);
            return Err(Error::new(ErrorKind::Other, e))
        }
    }
}
