use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{self, Formatter};
use std::hash::Hash;
use std::hash;
use std::sync::Arc;

use semver;
use serde::de;
use serde::ser;

use util::{CargoResult, CargoError, ToSemver};
use core::source::SourceId;

/// Identifier for a specific version of a package in a specific source.
#[derive(Clone)]
pub struct PackageId {
    inner: Arc<PackageIdInner>,
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct PackageIdInner {
    name: String,
    version: semver::Version,
    source_id: SourceId,
}

impl ser::Serialize for PackageId {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        let source = self.inner.source_id.to_url();
        let encoded = format!("{} {} ({})", self.inner.name, self.inner.version,
                              source);
        encoded.serialize(s)
    }
}

impl<'de> de::Deserialize<'de> for PackageId {
    fn deserialize<D>(d: D) -> Result<PackageId, D::Error>
        where D: de::Deserializer<'de>
    {
        let string = String::deserialize(d)?;
        let mut s = string.splitn(3, ' ');
        let name = s.next().unwrap();
        let version = match s.next() {
            Some(s) => s,
            None => return Err(de::Error::custom("invalid serialized PackageId")),
        };
        let version = semver::Version::parse(version)
                            .map_err(de::Error::custom)?;
        let url = match s.next() {
            Some(s) => s,
            None => return Err(de::Error::custom("invalid serialized PackageId")),
        };
        let url = if url.starts_with("(") && url.ends_with(")") {
            &url[1..url.len() - 1]
        } else {
            return Err(de::Error::custom("invalid serialized PackageId"))

        };
        let source_id = SourceId::from_url(url).map_err(de::Error::custom)?;

        Ok(PackageId {
            inner: Arc::new(PackageIdInner {
                name: name.to_string(),
                version: version,
                source_id: source_id,
            }),
        })
    }
}

impl Hash for PackageId {
    fn hash<S: hash::Hasher>(&self, state: &mut S) {
        self.inner.name.hash(state);
        self.inner.version.hash(state);
        self.inner.source_id.hash(state);
    }
}

impl PartialEq for PackageId {
    fn eq(&self, other: &PackageId) -> bool {
        (*self.inner).eq(&*other.inner)
    }
}
impl PartialOrd for PackageId {
    fn partial_cmp(&self, other: &PackageId) -> Option<Ordering> {
        (*self.inner).partial_cmp(&*other.inner)
    }
}
impl Eq for PackageId {}
impl Ord for PackageId {
    fn cmp(&self, other: &PackageId) -> Ordering {
        (*self.inner).cmp(&*other.inner)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PackageIdError {
    InvalidVersion(String),
    InvalidNamespace(String)
}

impl Error for PackageIdError {
    fn description(&self) -> &str { "failed to parse package id" }
}

impl fmt::Display for PackageIdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PackageIdError::InvalidVersion(ref v) => {
                write!(f, "invalid version: {}", *v)
            }
            PackageIdError::InvalidNamespace(ref ns) => {
                write!(f, "invalid namespace: {}", *ns)
            }
        }
    }
}

impl CargoError for PackageIdError {
    fn is_human(&self) -> bool { true }
}

impl From<PackageIdError> for Box<CargoError> {
    fn from(t: PackageIdError) -> Box<CargoError> { Box::new(t) }
}

impl PackageId {
    pub fn new<T: ToSemver>(name: &str, version: T,
                             sid: &SourceId) -> CargoResult<PackageId> {
        let v = version.to_semver().map_err(PackageIdError::InvalidVersion)?;
        Ok(PackageId {
            inner: Arc::new(PackageIdInner {
                name: name.to_string(),
                version: v,
                source_id: sid.clone(),
            }),
        })
    }

    pub fn name(&self) -> &str { &self.inner.name }
    pub fn version(&self) -> &semver::Version { &self.inner.version }
    pub fn source_id(&self) -> &SourceId { &self.inner.source_id }

    pub fn with_precise(&self, precise: Option<String>) -> PackageId {
        PackageId {
            inner: Arc::new(PackageIdInner {
                name: self.inner.name.to_string(),
                version: self.inner.version.clone(),
                source_id: self.inner.source_id.with_precise(precise),
            }),
        }
    }

    pub fn with_source_id(&self, source: &SourceId) -> PackageId {
        PackageId {
            inner: Arc::new(PackageIdInner {
                name: self.inner.name.to_string(),
                version: self.inner.version.clone(),
                source_id: source.clone(),
            }),
        }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} v{}", self.inner.name, self.inner.version)?;

        if !self.inner.source_id.is_default_registry() {
            write!(f, " ({})", self.inner.source_id)?;
        }

        Ok(())
    }
}

impl fmt::Debug for PackageId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("PackageId")
         .field("name", &self.inner.name)
         .field("version", &self.inner.version.to_string())
         .field("source", &self.inner.source_id.to_string())
         .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::PackageId;
    use core::source::SourceId;
    use sources::CRATES_IO;
    use util::ToUrl;

    #[test]
    fn invalid_version_handled_nicely() {
        let loc = CRATES_IO.to_url().unwrap();
        let repo = SourceId::for_registry(&loc);

        assert!(PackageId::new("foo", "1.0", &repo).is_err());
        assert!(PackageId::new("foo", "1", &repo).is_err());
        assert!(PackageId::new("foo", "bar", &repo).is_err());
        assert!(PackageId::new("foo", "", &repo).is_err());
    }
}
