use puremp3::Mp3Decoder;

#[test]
fn test_decode() -> Result<(), Box<std::error::Error>> {
    let data = std::fs::read("tests/vectors/MonoCBR192.mp3")?;
    let decoder = Mp3Decoder::new(&data[..]);
    decoder.frames().last();
    Ok(())
}
