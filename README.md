# Mlyr

`mlyr` is a Rust library designed to parse, validate, serialize, and embed synchronized lyrics. Its internal format uses XML and supports translation between standard LRC, word-level enhanced LRC, and Richsync (Musixmatch`s format)

> [!NOTE]
> Crate and format name originates from `Music LYRics`

## Features
 - Out-of-the-box conversion between LRC, eLRC, Richsync and own XML format
 - Data validation: lines cannot overlap and segments cannot have decreasing offset
 - Lyrics embedding: suport for embedding lyrics directly into audio file

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
      * `@url` *(Optional)*: URL of the **specific media**
* `<lyrics>`
  * `<line>`
    * `@offset` *(Optional)*: Start of the line in seconds
    * `@voice` *(Optional)*: Numeric identifier for the singer (1-255). Defaults to `0` (main)
    * `<segment>`: Container for text
      * `@offset`: Start of the segment **relative to the parent line** in seconds
      * `@text`
    * `<parallel>` *(Optional)*: Represents overlapping secondary vocals
      * `@offset`: Start relative to parent line
      * `@voice`
      * `<segment>`
        * `@offset`
        * `@text`

## Normalization

During parsing, `mlyr` automatically cleans and normalizes lyrics:

* Voice labels are mapped to sequential `u8` IDs (1-255)
* Lines are sorted chronologically
* Validation ensures no time overlaps occur between different lines and segments, and segments are in chronological order

---

## Usage

### Conversion Trait

`mlyr` uses the `LyricsConverter` trait to handle different formats. Formats supported out-of-the-box are `Lrc`, `Richsync`, and `Xml`

```rust
use mlyr::{Mlyr, Lrc, Richsync, Xml, LyricsConverter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lrc_data = "[00:12.00] Hello World";

    // Parse LRC into Mlyr
    let mlyr_instance = Lrc::from(lrc_data.to_string()).to_mlyr()?;

    // Convert Mlyr to Richsync JSON
    let richsync_json = Richsync::from_mlyr(mlyr_instance.clone())?;
    println!("{}", richsync_json);

    // Convert Mlyr to formatted XML
    let xml_str = Xml::from_mlyr_formatted(mlyr_instance)?;
    println!("{}", xml_str);

    Ok(())
}
```

### Creating object from scratch

Builders prevent writing invalid lyrics. Thanks to the `IntoOffset` trait, builders accept plain integers, floats, standard `Duration` objects, or even `()` (for no offset)

```rust
use std::time::Duration;
use mlyr::{MlyrBuilder, LineBuilder, Metadata};

fn create_valid_lyrics() -> Result<mlyr::Mlyr, mlyr::ValidationError> {
    let metadata = Metadata::new("Song Title", "Artist")
        .album("Greatest Hits")
        .length(Duration::from_secs(180));

    let line1 = LineBuilder::new(10.0)
        .voice(1)
        .add_segment(0.0, "Hello ".to_string())?
        .add_segment(2, "World".to_string())?
        .build();

    let lyrics = MlyrBuilder::new()
        .metadata(metadata)
        .add_line(line1)?
        .build()?;

    Ok(lyrics)
}
```

### Time-shifting offsets

Shifting line offsets inside the lyrics forward or backward:

```rust
use std::time::Duration;
use mlyr::Mlyr;

fn shift_lyrics(mlyr: &mut Mlyr) {
    let offset_delta = Duration::from_millis(500);
    mlyr.shift_timestamps(offset_delta, true);
}
```

### Embedding into audio files

Embedding the Mlyr XML format directly into audio file tags:

```rust
use mlyr::Mlyr;
use std::path::Path;

fn embed_lyrics_to_audio<P: AsRef<Path>>(mlyr: &Mlyr, path: P) -> Result<(), Box<dyn std::error::Error>> {
    mlyr.embed_to_file(&path)?;
    Ok(())
}

fn extract_lyrics_from_audio<P: AsRef<Path>>(path: P) -> Result<Mlyr, Box<dyn std::error::Error>> {
    let mlyr = Mlyr::from_file(&path)?;
    Ok(mlyr)
}
```
