# Mlyr

`mlyr` is a Rust library designed to parse, validate, serialize, and embed synchronized lyrics. Its format uses XML and supports translation between standard LRC, word-level enhanced LRC, and Musixmatch Richsync.

> [!NOTE]
> Crate and format name originates from `Music LYRics`

---

## Format
`mlyr` format uses XML and has this structure:

```xml
<mlyr>
  <metadata>
    <title>Song Title</title>
    <artist>Artist Name</artist>
    <album>Greatest Hits</album>
    <length>180</length>
    <sources>
      <source name="YouTube" url="youtube.com/watch?v=dQw4w9WgXcQ"/>
    </sources>
  </metadata>
  <lyrics>
    <line offset="10.0" voice="1">
      <segment offset="0.0" text="Hello "/>
      <segment offset="2.0" text="World"/>
      <parallel offset="11.0" voice="2">
        <segment offset="0.0" text="Hello (in background)"/>
      </parallel>
    </line>
    <line offset="15.0">
      <segment offset="0.0" text="Another line."/>
    </line>
  </lyrics>
</mlyr>
```

### Elements

* `<mlyr>`: Root
  * `<metadata>` *(Optional)*: Set to default/omitted if empty
    * `<title>` *(Optional)*
    * `<artist>` *(Optional)*
    * `<album>` *(Optional)*
    * `<length>` *(Optional)*: Duration of the song in seconds
    * `<sources>` *(Optional)*: Omitted if empty
      * `<source>`: Source of the media this lyric is meant for
        * `@name`: Name of the source platform (e.g., "Musixmatch")
        * `@url` *(Optional)*: URL of the **specific media** (e.g., `youtube.com/watch?v=dQw4w9WgXcQ`), and **not** the general platform site
  * `<lyrics>`
    * `<line>`
      * `@offset` *(Optional)*: Start of the line in seconds
      * `@voice` *(Optional)*: Numeric identifier for the singer (e.g., `1`, `2`). Defaults to `0` (main) and is omitted from XML when set to default
      * `<segment>`: Container for text
        * `@offset` *(Optional)*: Start of the segment **relative to the parent line** in seconds (defaults to `0`)
        * `@text`
      * `<parallel>` *(Optional)*: Represents overlapping secondary vocals
        * `@offset` *(Optional)*: Start of the line **relative to the parent line** in seconds (defaults to `0`)
        * `@voice` *(Optional)*
        * `<segment>`
          * `@offset` *(Optional)*
          * `@text`

## Normalization

During parsing, `mlyr` automatically cleans and normalizes lyrics:
* Any arbitrary voice labels found in the input (like string identifiers `"Lead"` / `"Backup"` or large integers) are mapped to sequential `u8` IDs starting from `1` (capping at `255`), preserving `0` as the default
* Lines and word segments that are out-of-order in the raw input are sorted chronologically

---

## Usage

### Parsing and writing LRC

You can import standard or word-enhanced LRC files and convert them into the internal model, or export an existing model back to LRC format:

```rust
use mlyr::{Mlyr, ConversionError};

fn main() -> Result<(), ConversionError> {
    let lrc_data = r#"
    [ti: Song Title]
    [ar: Artist Name]
    [00:12.00] This is a <00:12.50> simple <00:13.00> lyric line.
    [00:15.00] Another line.
    "#;

    // Parsing
    let mlyr_instance = Mlyr::from_lrc(lrc_data)?;

    // Converting back
    let lrc_output = mlyr_instance.to_lrc()?;
    println!("{}", lrc_output);

    Ok(())
}
```

### Validating lyrics with builders

Builders prevent writing invalid lyrics. Thanks to the `IntoOffset` trait, builders accept plain integers, floats, standard `Duration` objects, or even `()` (for no offset):

```rust
use std::time::Duration;
use mlyr::{MlyrBuilder, LineBuilder, Metadata};

fn create_valid_lyrics() -> Result<mlyr::Mlyr, mlyr::ValidationError> {
    let metadata = Metadata::new("Song Title", "Artist")
        .album("Greatest Hits")
        .length(Duration::from_secs(180));

    // Builders accept generic numeric seconds or Duration types seamlessly
    let line1 = LineBuilder::new(10.0)
        .voice(1)
        .add_segment(0.0, "Hello ".to_string())?
        .add_segment(2, "World".to_string())?
        .build();

    // The builder checks that lines and segments go after each other
    let lyrics = MlyrBuilder::new()
        .metadata(metadata)
        .add_line(line1)?
        .build()?;

    Ok(lyrics)
}
```

### Time-shifting offsets

You can shift all timestamps inside the lyrics forward (delaying) or backward (advancing) by a specific duration:

```rust
use std::time::Duration;
use mlyr::Mlyr;

fn shift_lyrics(mlyr: &mut Mlyr) {
    let offset_delta = Duration::from_millis(500);
    
    // Shift all lyric lines and segment timestamps forward by 0.5 seconds
    mlyr.shift_timestamps(offset_delta, true);
}
```

### XML serialization

`Mlyr` structures can be serialized to minified XML or indented XML:

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

Translate from Musixmatch's character/word synchronized JSON schema to `Mlyr` and vice versa:

```rust
use mlyr::{Mlyr, ConversionError};

fn handle_richsync(json_content: &str) -> Result<(), ConversionError> {
    // Parsing
    let mlyr = Mlyr::from_richsync(json_content)?;

    // Converting back
    let richsync_json = mlyr.to_richsync()?;
    println!("{}", richsync_json);

    Ok(())
}
```

### Embedding into audio files

You can embed this format directly into an audio file's tag metadata, or extract it back out:

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
