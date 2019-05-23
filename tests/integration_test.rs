use puremp3::Mp3Iterator;

#[test]
fn test_decode() -> Result<(), Box<std::error::Error>> {
    let data = std::fs::read("tests/vectors/MonoCBR192.mp3")?;
    let iter = Mp3Iterator::new(&data[..]);
    iter.last();
    Ok(())
}
