use chrono::prelude::*;
use directories::BaseDirs;
use itertools::Itertools;
use percent_encoding::percent_decode;
use quick_xml::events::attributes::{Attribute, Attributes};
use quick_xml::events::Event;
use quick_xml::{Reader, Writer};
use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{rename, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::result::Result;
use std::str;
use std::vec::Vec;

#[derive(Debug)]
struct NoBaseDirsError;
impl fmt::Display for NoBaseDirsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoBaseDirsError")
    }
}
impl Error for NoBaseDirsError {}

#[derive(Debug)]
struct BookmarkWithoutSingleHrefError;
impl fmt::Display for BookmarkWithoutSingleHrefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BookmarkWithoutSingleHrefError")
    }
}
impl Error for BookmarkWithoutSingleHrefError {}

#[derive(Debug)]
struct HrefNotRecognizedError {
    href: String,
}
impl fmt::Display for HrefNotRecognizedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HrefNotRecognizedError: {}", self.href)
    }
}
impl Error for HrefNotRecognizedError {}

fn href_attribute(attributes: Attributes) -> Result<Cow<'_, [u8]>, BookmarkWithoutSingleHrefError> {
    attributes
        .filter_map(|a| match a {
            Ok(Attribute {
                key: b"href",
                value,
            }) => Some(value),
            _ => None,
        })
        .exactly_one()
        .map_err(|_e| BookmarkWithoutSingleHrefError)
}

fn path_needs_cleaning(paths_to_clean: &[String], path: &str) -> bool {
    paths_to_clean
        .iter()
        .any(|path_to_clean| path.starts_with(path_to_clean))
}

fn read_filter_write<R: BufRead, W: Write>(
    reader: R,
    writer: W,
    paths_to_clean: &[String],
) -> Result<(), Box<dyn Error>> {
    let mut reader = Reader::from_reader(reader);
    let mut buf = Vec::new();

    let mut writer = Writer::new(writer);

    let mut skipping = false;
    let mut skip_whitespace = false;

    loop {
        if skipping {
            match reader.read_event(&mut buf) {
                Ok(Event::End(e)) if e.name() == b"bookmark" => {
                    skipping = false;
                    skip_whitespace = true;
                }
                _ => (),
            }
        } else {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(e)) => {
                    if e.name() == b"bookmark" {
                        let attr = href_attribute(e.attributes())?;
                        let href = percent_decode(&attr).decode_utf8()?;
                        if let Some(path) = href.strip_prefix("file://") {
                            if path_needs_cleaning(&paths_to_clean, &path) {
                                skipping = true;
                                continue;
                            }
                        } else if let Some(_) = href.strip_prefix("trash://") {
                            // do nothing
                        } else if let Some(_) = href.strip_prefix("mtp://") {
                            // do nothing
                        } else {
                            return Err(Box::new(HrefNotRecognizedError {
                                href: href.to_string(),
                            }));
                        };
                    }
                    writer.write_event(Event::Start(e))?;
                }
                Ok(Event::End(e)) => {
                    writer.write_event(Event::End(e))?;
                }
                Ok(Event::Empty(e)) => {
                    writer.write_event(Event::Empty(e))?;
                }
                Ok(Event::Text(e)) => {
                    if skip_whitespace {
                        skip_whitespace = false;
                        assert!(e
                            .unescape_and_decode(&reader)?
                            .chars()
                            .all(char::is_whitespace));
                    } else {
                        writer.write_event(Event::Text(e))?;
                    }
                }
                Ok(Event::Eof) => break,
                Ok(Event::Decl(e)) => {
                    writer.write_event(Event::Decl(e))?;
                }
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                other => unimplemented!("{:?}", other),
            }
        }
    }
    writer.into_inner().flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let base_dirs = BaseDirs::new().ok_or(NoBaseDirsError)?;
    let dir = base_dirs.data_dir();
    let input_filename = dir.join("recently-used.xbel");
    let output_filename = dir.join(Local::now().format("recently-used.xbel-%+").to_string());
    let paths_to_clean: Vec<String> = env::args().skip(1).collect();

    let input_file = File::open(&input_filename)?;
    let reader = BufReader::new(input_file);

    let output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_filename)?;
    let writer = BufWriter::new(output_file);

    read_filter_write(reader, writer, &paths_to_clean)?;

    rename(output_filename, input_filename)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn no_filter() {
        let input = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        let mut output = Vec::new();
        read_filter_write(BufReader::new(input.as_bytes()), &mut output, &vec![]).unwrap();
        assert_eq!(input, String::from_utf8(output).unwrap());
    }

    #[test]
    fn filter_two() {
        let input = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///home/a/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
  <bookmark href="file:///home/b/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        let mut output = Vec::new();
        read_filter_write(
            BufReader::new(input.as_bytes()),
            &mut output,
            &vec![String::from("/home/a"), String::from("/home/b")],
        )
        .unwrap();
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        assert_eq!(expected, String::from_utf8(output).unwrap());
    }

    #[test]
    fn filter_one() {
        let input = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///tmp/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        let mut output = Vec::new();
        read_filter_write(
            BufReader::new(input.as_bytes()),
            &mut output,
            &vec![String::from("/tmp")],
        )
        .unwrap();
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        assert_eq!(expected, String::from_utf8(output).unwrap());
    }

    #[test]
    fn filter_encoded() {
        let input = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///opt/A%20Directory/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        let mut output = Vec::new();
        read_filter_write(
            BufReader::new(input.as_bytes()),
            &mut output,
            &vec![String::from("/opt/A Directory")],
        )
        .unwrap();
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info"
>
  <bookmark href="file:///home/me/A-File.txt" added="2020-09-24T20:00:00Z" modified="2020-09-25T20:00:00Z" visited="2020-09-25T20:00:00Z">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="text/plain"/>
        <bookmark:groups>
          <bookmark:group>gedit</bookmark:group>
        </bookmark:groups>
        <bookmark:applications>
          <bookmark:application name="gedit" exec="&apos;gedit %u&apos;" modified="2020-09-25T20:00:00Z" count="1234"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
</xbel>
"#;
        assert_eq!(expected, String::from_utf8(output).unwrap());
    }
}
