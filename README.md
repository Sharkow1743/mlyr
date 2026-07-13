# Mlyr

`mlyr` is a Rust library designed to parse, validate, serialize, and embed synchronized lyrics. Its format uses xml and supports translation between standard LRC, word-level enhanced LRC and Musixmatch Richsync

---

## Usage examples

### Parsing and writing LRC

You can import standard or word-enhanced LRC files and convert them into the internal model, or export an existing model back to LRC format

```rust
use mlyr::Mlyr;

fn main() -> Result<(), String> {
    let lrc_data = r#"
    [ti: Song Title]
    [ar: Artist Name]
    [00:12.00] This is a <00:12.50> simple <00:13.00> lyric line.
    [00:15.00] Another line.
    "#;

    // Parsing
    let mlyr_instance = Mlyr::from_lrc(lrc_data)?;

    // Converting back
    let lrc_output = mlyr_instance.to_lrc();
    println!("{}", lrc_output);

    Ok(())
}
```

### Validating lyrics with builders

Builders prevent writing invalid lyrics

```rust
use std::time::Duration;
use mlyr::{MlyrBuilder, LineBuilder, Metadata};

fn create_valid_lyrics() -> Result<mlyr::Mlyr, mlyr::ValidationError> {
    let metadata = Metadata::new("Song Title", "Artist")
        .album("Greatest Hits")
        .length(Duration::from_secs(180));

    let line1 = LineBuilder::new(Some(Duration::from_secs(10)))
        .voice("Lead")
        .add_segment(Some(Duration::from_secs(0)), "Hello ".to_string())?
        .add_segment(Some(Duration::from_secs(2)), "World".to_string())?
        .build();

    // The builder checks that this line doesn't start before the previous line's end bound
    let lyrics = MlyrBuilder::new()
        .metadata(metadata)
        .add_line(line1)?
        .build();

    Ok(lyrics)
}
```

### XML serialization

`Mlyr` structures can be serialized to minified XML or indented XML

```rust
use mlyr::Mlyr;

fn to_xml_string(mlyr: &Mlyr) -> Result<(), Box<dyn std::error::Error>> {
    // Minified XML
    let xml = mlyr.to_xml()?;
    println!("{}", xml);

    // Indented XML
    let formatted_xml = mlyr.to_xml_formatted()?;
    println!("{}", formatted_xml);

    Ok(())
}
```

### Musixmatch richsync conversion

Translate from musixmatch's character/word synchronized JSON schema to `Mlyr` and vice versa

```rust
use mlyr::Mlyr;

fn handle_richsync(json_content: &str) -> Result<(), serde_json::Error> {
    // Parsing
    let mlyr = Mlyr::from_richsync(json_content)?;

    // Converting back
    let richsync_json = mlyr.to_richsync()?;
    println!("{}", richsync_json);

    Ok(())
}
```

### Embedding into audio files

You can embed this format directly into an audio file's tag metadata, or extract it back out

```rust
use mlyr::Mlyr;
use std::path::Path;

fn embed_lyrics_to_audio<P: AsRef<Path>>(mlyr: &Mlyr, path: P) -> Result<(), Box<dyn std::error::Error>> {
    // Write to metadata of an audio file
    mlyr.embed_to_file(&path)?;
    println!("Lyrics embedded successfully.");
    Ok(())
}

fn extract_lyrics_from_audio<P: AsRef<Path>>(path: P) -> Result<Mlyr, Box<dyn std::error::Error>> {
    // Read from metadata
    let mlyr = Mlyr::from_file(&path)?;
    Ok(mlyr)
}
```
