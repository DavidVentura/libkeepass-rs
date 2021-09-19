use crate::crypt::ciphers::Cipher;
use crate::result::{DatabaseIntegrityError, Error, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use std::collections::HashMap;
use xmltree::{Element, XMLNode};

use secstr::SecStr;

use xml::writer::{EmitterConfig, EventWriter, Result as WResult, XmlEvent as WXmlEvent};

use std::io::Write;

use super::db::{AutoType, AutoTypeAssociation, Database, Entry, Group, Meta, Node, Value};

pub(crate) trait Serializable {
    fn serialize<W: Write>(
        &self,
        w: &mut EventWriter<W>,
        encryptor: &mut dyn Cipher,
    ) -> WResult<()>;
}

fn write_simple_element<W: Write>(w: &mut EventWriter<W>, tag: &str, value: &str) -> WResult<()> {
    w.write(WXmlEvent::start_element(tag))?;
    w.write(WXmlEvent::characters(value))?;
    w.write(WXmlEvent::end_element())?;
    Ok(())
}

impl Serializable for Meta {
    fn serialize<W: Write>(
        &self,
        w: &mut EventWriter<W>,
        _encryptor: &mut dyn Cipher,
    ) -> WResult<()> {
        w.write(WXmlEvent::start_element("Meta"))?;

        w.write(WXmlEvent::start_element("RecycleBinUUID"))?;
        w.write(WXmlEvent::characters(&self.recyclebin_uuid))?;
        w.write(WXmlEvent::end_element())?;

        for (k, v) in &self.unhandled_fields {
            w.write(WXmlEvent::start_element(k.as_str()))?;
            w.write(WXmlEvent::characters(v.as_str()))?;
            w.write(WXmlEvent::end_element())?;
        }

        w.write(WXmlEvent::end_element())?;
        Ok(())
    }
}

impl Serializable for Entry {
    fn serialize<W: Write>(
        &self,
        w: &mut EventWriter<W>,
        encryptor: &mut dyn Cipher,
    ) -> WResult<()> {
        w.write(WXmlEvent::start_element("Entry"))?;

        w.write(WXmlEvent::start_element("UUID"))?;
        w.write(WXmlEvent::characters(&self.uuid))?;
        w.write(WXmlEvent::end_element())?;

        for (k, v) in &self.unhandled_fields {
            w.write(WXmlEvent::start_element(k.as_str()))?;
            w.write(WXmlEvent::characters(v.as_str()))?;
            w.write(WXmlEvent::end_element())?;
        }

        for field_name in self.fields.keys() {
            w.write(WXmlEvent::start_element("String"))?;
            w.write(WXmlEvent::start_element("Key"))?;
            w.write(WXmlEvent::characters(&field_name))?;
            w.write(WXmlEvent::end_element())?;
            match self.fields.get(field_name) {
                Some(&Value::Bytes(_)) => {
                    w.write(WXmlEvent::start_element("Value"))?;
                    // FIXME: no bytes value
                    w.write(WXmlEvent::end_element())?;
                }
                Some(&Value::Protected(ref pv)) => {
                    w.write(WXmlEvent::start_element("Value").attr("Protected", "True"))?;

                    let plain = std::str::from_utf8(pv.unsecure())
                        .ok()
                        .unwrap()
                        .as_bytes()
                        .to_vec();

                    if plain.len() > 0 {
                        let buf_encrypted = encryptor.encrypt(&plain).unwrap();
                        let buf_encoded = base64::encode(&buf_encrypted);

                        w.write(WXmlEvent::characters(&buf_encoded))?;
                    }
                    w.write(WXmlEvent::end_element())?;
                }
                Some(&Value::Unprotected(ref uv)) => {
                    w.write(WXmlEvent::start_element("Value"))?;
                    w.write(WXmlEvent::characters(&uv))?;
                    w.write(WXmlEvent::end_element())?;
                }
                None => {
                    w.write(WXmlEvent::start_element("Value"))?;
                    w.write(WXmlEvent::end_element())?;
                }
            };

            w.write(WXmlEvent::end_element())?;
        }

        if let Some(at) = &self.autotype {
            w.write(WXmlEvent::start_element("AutoType"))?;
            write_simple_element(w, "Enabled", if at.enabled { "True" } else { "False" })?;
            if let Some(seq) = &at.sequence {
                write_simple_element(w, "DefaultSequence", seq)?;
            }
            w.write(WXmlEvent::end_element())?;
        }

        if self.history.len() > 0 {
            w.write(WXmlEvent::start_element("History"))?;
            for history_item in &self.history {
                history_item.serialize(w, encryptor)?;
            }
            w.write(WXmlEvent::end_element())?;
        }

        let start =
            chrono::NaiveDateTime::parse_from_str("0001-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap()
                .timestamp();
        w.write(WXmlEvent::start_element("Times"))?;
        write_simple_element(w, "Expires", if self.expires { "True" } else { "False" })?;

        for (key, value) in &self.times {
            let mut ts_bytes = vec![];
            ts_bytes.write_i64::<LittleEndian>(value.timestamp() - start)?;
            w.write(WXmlEvent::start_element(key.as_str()))?;
            w.write(WXmlEvent::characters(base64::encode(ts_bytes).as_str()))?;
            w.write(WXmlEvent::end_element())?;
        }
        w.write(WXmlEvent::end_element())?;

        w.write(WXmlEvent::end_element())?;
        Ok(())
    }
}
impl Serializable for Group {
    fn serialize<W: Write>(
        &self,
        w: &mut EventWriter<W>,
        encryptor: &mut dyn Cipher,
    ) -> WResult<()> {
        w.write(WXmlEvent::start_element("Group"))?;

        w.write(WXmlEvent::start_element("UUID"))?;
        w.write(WXmlEvent::characters(&self.uuid))?;
        w.write(WXmlEvent::end_element())?;

        w.write(WXmlEvent::start_element("Name"))?;
        w.write(WXmlEvent::characters(&self.name))?;
        w.write(WXmlEvent::end_element())?;

        for (k, v) in &self.unhandled_fields {
            w.write(WXmlEvent::start_element(k.as_str()))?;
            w.write(WXmlEvent::characters(v.as_str()))?;
            w.write(WXmlEvent::end_element())?;
        }

        let start =
            chrono::NaiveDateTime::parse_from_str("0001-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap()
                .timestamp();
        w.write(WXmlEvent::start_element("Times"))?;
        write_simple_element(w, "Expires", if self.expires { "True" } else { "False" })?;
        for (key, value) in &self.times {
            let mut ts_bytes = vec![];
            ts_bytes.write_i64::<LittleEndian>(value.timestamp() - start)?;
            w.write(WXmlEvent::start_element(key.as_str()))?;
            w.write(WXmlEvent::characters(base64::encode(ts_bytes).as_str()))?;
            w.write(WXmlEvent::end_element())?;
        }
        w.write(WXmlEvent::end_element())?;

        for node in &self.children {
            match node {
                Node::Group(g) => g.serialize(w, encryptor)?,
                Node::Entry(e) => e.serialize(w, encryptor)?,
            };
        }

        w.write(WXmlEvent::end_element())?;
        Ok(())
    }
}

impl Serializable for Database {
    fn serialize<W: Write>(
        &self,
        w: &mut EventWriter<W>,
        encryptor: &mut dyn Cipher,
    ) -> WResult<()> {
        w.write(WXmlEvent::start_element("KeePassFile"))?;
        self.meta.serialize(w, encryptor)?;
        w.write(WXmlEvent::start_element("Root"))?;
        self.root.serialize(w, encryptor)?;
        w.write(WXmlEvent::end_element())?;
        w.write(WXmlEvent::end_element())?;
        Ok(())
    }
}

fn parse_xml_timestamp(t: &str) -> Result<chrono::NaiveDateTime> {
    match chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%SZ") {
        // Prior to KDBX4 file format, timestamps were stored as ISO 8601 strings
        Ok(ndt) => Ok(ndt),
        // In KDBX4, timestamps are stored as seconds, Base64 encoded, since 0001-01-01 00:00:00
        // So, if we don't have a valid ISO 8601 string, assume we have found a Base64 encoded int.
        _ => {
            let v = base64::decode(t).map_err(|e| Error::from(DatabaseIntegrityError::from(e)))?;
            // Cast the Vec created by base64::decode into the array expected by i64::from_le_bytes
            let mut a: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
            a.copy_from_slice(&v[0..8]);
            let sec = i64::from_le_bytes(a);
            let ndt =
                chrono::NaiveDateTime::parse_from_str("0001-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S")
                    .unwrap()
                    + chrono::Duration::seconds(sec);
            Ok(ndt)
        }
    }
}

pub(crate) fn write_xml(d: &Database, encryptor: &mut dyn Cipher) -> WResult<Vec<u8>> {
    let mut data = Vec::new();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut data);

    d.serialize(&mut writer, encryptor).unwrap();
    Ok(data)
}

fn parse_meta(e: &Element) -> Meta {
    let mut meta = Meta {
        ..Default::default()
    };
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "RecycleBinUUID" => meta.recyclebin_uuid = get_text(el),
                "MemoryProtection" => (), // FIXME
                "CustomData" => (),       // FIXME
                _ => {
                    meta.unhandled_fields.insert(el.name.clone(), get_text(el));
                }
            }
        }
    }
    meta
}

fn parse_history(e: &Element, inner_cipher: &mut dyn Cipher) -> Vec<Entry> {
    let mut res = Vec::new();
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "Entry" => res.push(parse_entry(el, inner_cipher)),
                _ => panic!("Found {} when parsing history", el.name),
            }
        }
    }
    res
}
fn parse_autotype(e: &Element) -> Option<AutoType> {
    let mut at = AutoType {
        ..Default::default()
    };
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "Enabled" => at.enabled = get_text(el) == "True",
                "DefaultSequence" => at.sequence = Some(get_text(el)),
                "DataTransferObfuscation" => (), // FIXME
                "Association" => {
                    let a_hm = get_hashmap(el);
                    at.associations.push(AutoTypeAssociation {
                        window: a_hm.get("Window").map(String::to_owned),
                        sequence: a_hm.get("KeystrokeSequence").map(String::to_owned),
                    });
                }
                _ => panic!(
                    "Found unhandled element {} when parsing autotype for {:?}",
                    el.name, e
                ),
            }
        }
    }
    Some(at)
}

fn get_hashmap(e: &Element) -> HashMap<String, String> {
    let mut ret = HashMap::new();
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            ret.insert(el.name.clone(), get_text(el));
        }
    }
    ret
}
fn get_kv_pair(e: &Element, inner_cipher: &mut dyn Cipher) -> (String, Value) {
    let mut key: Option<String> = None;
    let mut val: Option<Value> = None;

    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "Key" => key = Some(get_text(el)),
                "Value" => {
                    if let Some(p) = el.attributes.get("Protected") {
                        if p == "True" {
                            let enc_bytes = base64::decode(get_text(el)).unwrap();
                            let dec_bytes = inner_cipher.decrypt(&enc_bytes).unwrap();
                            val = Some(Value::Protected(SecStr::new(dec_bytes)));
                            break;
                        }
                    }
                    val = Some(Value::Unprotected(get_text(el)));
                }
                _ => panic!("Found el {} when parsing KV pair", el.name),
            }
        }
    }

    // blowing up if no key/value are found
    (key.unwrap(), val.unwrap())
}

fn parse_entry(e: &Element, inner_cipher: &mut dyn Cipher) -> Entry {
    let mut entry = Entry {
        ..Default::default()
    };
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "UUID" => entry.uuid = get_text(el),
                "Times" => {
                    let (t, e) = parse_times(el);
                    entry.times = t;
                    entry.expires = e;
                }
                "String" => {
                    let (k, v) = get_kv_pair(el, inner_cipher);
                    entry.fields.insert(k, v);
                }
                "AutoType" => entry.autotype = parse_autotype(el),
                "History" => entry.history = parse_history(el, inner_cipher),
                "CustomData" => (), // FIXME
                _ => {
                    entry.unhandled_fields.insert(el.name.clone(), get_text(el));
                }
            }
        }
    }
    entry
}
fn get_text(e: &Element) -> String {
    let mut _s: String;
    for node in &e.children {
        if let XMLNode::Text(s) = node {
            return s.clone();
        }

        panic!(
            "Found a non-text child item when parsing {:?} - {:?}",
            node, e
        );
    }
    "".to_string()
}

fn parse_times(e: &Element) -> (HashMap<String, chrono::NaiveDateTime>, bool) {
    let mut times = HashMap::new();
    let mut expires = false;

    let time_fields = [
        "LastModificationTime",
        "CreationTime",
        "LastAccessTime",
        "ExpiryTime",
        "LocationChanged",
    ];
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            if time_fields.contains(&el.name.as_str()) {
                let ts = parse_xml_timestamp(&get_text(el)).unwrap();
                times.insert(el.name.clone(), ts);
            } else if el.name == "Expires" {
                expires = get_text(el) == "True";
            }
            // FIXME: other
            //		<Expires>False</Expires>
            //		<UsageCount>0</UsageCount>
        }
    }
    (times, expires)
}

fn parse_group(e: &Element, inner_cipher: &mut dyn Cipher) -> Group {
    let mut group = Group {
        ..Default::default()
    };

    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "UUID" => group.uuid = get_text(el),
                "Name" => group.name = get_text(el),
                "Group" => group
                    .children
                    .push(Node::Group(parse_group(el, inner_cipher))),
                "Entry" => group
                    .children
                    .push(Node::Entry(parse_entry(el, inner_cipher))),
                "Times" => {
                    let (t, e) = parse_times(el);
                    group.times = t;
                    group.expires = e;
                }
                _ => {
                    group.unhandled_fields.insert(el.name.clone(), get_text(el));
                }
            }
        }
    }
    group
}
fn parse_root(e: &Element, inner_cipher: &mut dyn Cipher) -> Group {
    let mut root = Group {
        ..Default::default()
    };
    for node in &e.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "Group" => root = parse_group(el, inner_cipher),
                _ => println!("<root> Found unknown element! {}", el.name),
            }
        }
    }
    root
}
pub(crate) fn parse_xml_block(xml: &[u8], inner_cipher: &mut dyn Cipher) -> Result<(Group, Meta)> {
    let mut root_group: Group = Default::default();
    let mut meta: Meta = Default::default();
    let root_el = Element::parse(xml).unwrap();
    for node in &root_el.children {
        if let XMLNode::Element(el) = node {
            match el.name.as_str() {
                "Root" => root_group = parse_root(el, inner_cipher),
                "Meta" => meta = parse_meta(el),
                _ => panic!("Found unknown element! {}", el.name),
            }
        }
    }
    Ok((root_group, meta))
}
