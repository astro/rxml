/*!
# Strongly-typed strings for use with XML 1.0 documents

This module defines various string- and str-like types which represent pieces
of text as they may occur in XML documents. These types are checked to contain
only text which conforms to the respective grammar in the XML specifications.

This allows to carry information about the checking which already took place
in the parser to the application, avoiding the need to execute checks multiple
times.

## Type Overview

- [`Name`] and [`NameStr`] represent the `Name` production and can be used
  for element and attribute names before namespace prefix expansion.
- [`NCName`] and [`NCNameStr`] represent the `Name` production but without a
  colon inside; they are used for localnames after prefix expansion and to
  carry the prefixes themselves.
- [`CData`] and [`CDataStr`] represent strings of XML `Char`s, which are
  slightly more restrictive than Rust `char`. They are used for attribute
  values and text nodes.

  Note that [`CData`] strings do not contain references or CDATA sections;
  those are expanded by the lexer.

## Construction

To construct [`str`]-like references at compile time, you can use the macros
from the [`rxml_proc`](https://docs.rs/rxml_proc) crate. It offers
`xml_cdata!`, `xml_name!` and `xml_ncname!` macros which convert a string
literal into `&CDataStr`, `&NameStr` and `&NCNameStr` (respectively) with
validation at compile time.

In general, owned values are constructed using the [`std::convert::TryInto`]
mechanism, from other string types. Supported source types are:

* [`String`] (copies for [`Name`] and [`NCName`], moves for [`CData`])
* [`smartstring::alias::String`] (copies for [`CData`], moves for [`Name`] and [`NCName`])
* [`str`] (copies for all types except the slice types)

In addition, the following conversions can be done without extra checking and
are possible through `.into()`:

* [`NCName`] to [`Name`]
* [`NCName`] to [`CData`]
* [`Name`] to [`CData`]

(and likewise for the corresponding Str types)

The inverse directions are only available through `try_into`.
*/

use crate::error::{NWFError, WFError, ERRCTX_UNKNOWN};
use rxml_validation::selectors;
use rxml_validation::selectors::CharSelector;
pub use rxml_validation::{validate_cdata, validate_name, validate_ncname};
use smartstring::alias::String as SmartString;
use std::borrow::{Borrow, Cow, ToOwned};
use std::cmp::{Ordering, PartialOrd};
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::ops::{Add, Deref};

macro_rules! rxml_unsafe_str_construct_doc {
	($name:ident, $other:ident) => {
		concat!(
			"Construct a `",
			stringify!($name),
			"` without enforcing anything\n",
			"\n",
			"# Safety\n",
			"\n",
			"The caller is responsible for ensuring that the passed [`",
			stringify!($other),
			"`] is in fact a valid `",
			stringify!($name),
			"`.\n",
		)
	};
}

macro_rules! rxml_safe_str_construct_doc {
	($name:ident, $other:ident, $more:expr) => {
		concat!(
			"Converts a [`",
			stringify!($other),
			"`] to a `",
			stringify!($name),
			"`.\n",
			"\n",
			"If the given `",
			stringify!($other),
			"` does not conform to the restrictions imposed by `",
			stringify!($name),
			"`, an error is returned.\n",
			$more
		)
	};
}

macro_rules! rxml_custom_string_type {
	(
		$(#[$outer:meta])*
		pub struct $name:ident($string:ty) use $check:ident => $borrowed:ident;
	) => {
		$(#[$outer])*
		#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord)]
		#[repr(transparent)]
		pub struct $name($string);

		impl $name {
			#[deprecated(since = "0.4.0", note = "use the TryFrom<> trait implementation instead")]
			#[doc = rxml_safe_str_construct_doc!($name, str, "")]
			pub fn from_str<T: AsRef<str>>(s: T) -> Result<Self, WFError> {
				s.as_ref().try_into()
			}

			#[deprecated(since = "0.4.0", note = "use the TryFrom<> trait implementation instead")]
			#[doc = rxml_safe_str_construct_doc!($name, String, "")]
			pub fn from_string<T: Into<String>>(s: T) -> Result<Self, WFError> {
				s.into().try_into()
			}

			#[deprecated(since = "0.4.0", note = "use the TryFrom<> trait implementation instead")]
			#[doc = rxml_safe_str_construct_doc!($name, SmartString, "")]
			pub fn from_smartstring<T: Into<SmartString>>(s: T) -> Result<Self, WFError> {
				s.into().try_into()
			}

			/// Extract the inner string and return it.
			pub fn into_inner(self) -> $string {
				self.0
			}

			/// Obtain a reference to the inner string slice.
			pub fn as_str(&self) -> &str {
				self.0.as_str()
			}

			#[doc = rxml_unsafe_str_construct_doc!($name, str)]
			pub unsafe fn from_str_unchecked<T: AsRef<str>>(s: T) -> Self {
				Self(s.as_ref().into())
			}

			#[doc = rxml_unsafe_str_construct_doc!($name, String)]
			pub unsafe fn from_string_unchecked<T: Into<String>>(s: T) -> Self {
				Self(s.into().into())
			}

			#[doc = rxml_unsafe_str_construct_doc!($name, SmartString)]
			pub unsafe fn from_smartstring_unchecked<T: Into<SmartString>>(s: T) -> Self {
				Self(s.into().into())
			}

			unsafe fn from_native_unchecked(s: $string) -> Self {
				Self(s)
			}
		}

		impl Deref for $name {
			type Target = $borrowed;

			fn deref(&self) -> &Self::Target {
				// SAFETY: $borrowed is assumed to use the same check; this is
				// enforced by using the pair macro.
				unsafe { $borrowed::from_str_unchecked(&self.0) }
			}
		}

		impl Borrow<$string> for $name {
			fn borrow(&self) -> &$string {
				&self.0
			}
		}

		impl Borrow<$borrowed> for $name {
			fn borrow(&self) -> &$borrowed {
				// SAFETY: $borrowed is assumed to use the same check; this is
				// enforced by using the pair macro.
				unsafe { $borrowed::from_str_unchecked(&self.0) }
			}
		}

		impl Borrow<str> for $name {
			fn borrow(&self) -> &str {
				&self.0
			}
		}

		impl AsRef<$string> for $name {
			fn as_ref(&self) -> &$string {
				&self.0
			}
		}

		impl AsRef<$borrowed> for $name {
			fn as_ref(&self) -> &$borrowed {
				// SAFETY: $borrowed is assumed to use the same check; this is
				// enforced by using the pair macro.
				unsafe { $borrowed::from_str_unchecked(&self.0) }
			}
		}

		impl AsRef<str> for $name {
			fn as_ref(&self) -> &str {
				&self.0
			}
		}

		impl PartialEq<str> for $name {
			fn eq(&self, other: &str) -> bool {
				&self.0 == other
			}
		}

		// following the example of std::string::String, we define PartialEq
		// against the slice and the base type.
		impl PartialEq<$name> for str {
			fn eq(&self, other: &$name) -> bool {
				other.0 == self
			}
		}

		impl PartialEq<&str> for $name {
			fn eq(&self, other: &&str) -> bool {
				&self.0 == *other
			}
		}

		impl PartialEq<$name> for &str {
			fn eq(&self, other: &$name) -> bool {
				other.0 == *self
			}
		}

		impl PartialEq<$borrowed> for $name {
			fn eq(&self, other: &$borrowed) -> bool {
				self.0 == other.0
			}
		}

		impl PartialEq<$name> for $borrowed {
			fn eq(&self, other: &$name) -> bool {
				other.0 == self.0
			}
		}

		impl PartialEq<&$borrowed> for $name {
			fn eq(&self, other: &&$borrowed) -> bool {
				self.0 == other.0
			}
		}

		impl PartialEq<$name> for &$borrowed {
			fn eq(&self, other: &$name) -> bool {
				other.0 == self.0
			}
		}

		impl PartialOrd<$name> for $name {
			fn partial_cmp(&self, other: &$name) -> Option<Ordering> {
				self.0.partial_cmp(&other.0)
			}
		}

		impl From<$name> for String {
			fn from(other: $name) -> Self {
				other.0.into()
			}
		}

		impl From<$name> for SmartString {
			fn from(other: $name) -> Self {
				other.0.into()
			}
		}

		impl<'x> From<$name> for Cow<'x, $borrowed> {
			fn from(other: $name) -> Self {
				Self::Owned(other)
			}
		}

		impl<'x> From<Cow<'x, $borrowed>> for $name {
			fn from(other: Cow<'x, $borrowed>) -> Self {
				other.into_owned()
			}
		}

		impl TryFrom<SmartString> for $name {
			type Error = WFError;

			#[doc = rxml_safe_str_construct_doc!($name, SmartString, "")]
			fn try_from(other: SmartString) -> Result<Self, Self::Error> {
				$check(&other)?;
				Ok($name(other.into()))
			}
		}

		impl TryFrom<String> for $name {
			type Error = WFError;

			#[doc = rxml_safe_str_construct_doc!($name, String, "")]
			fn try_from(other: String) -> Result<Self, Self::Error> {
				$check(&other)?;
				Ok($name(other.into()))
			}
		}

		impl TryFrom<&str> for $name {
			type Error = WFError;

			#[doc = rxml_safe_str_construct_doc!($name, str, "")]
			fn try_from(other: &str) -> Result<Self, Self::Error> {
				$check(other)?;
				Ok($name(other.into()))
			}
		}

		impl fmt::Display for $name {
			fn fmt<'f>(&self, f: &'f mut fmt::Formatter) -> fmt::Result {
				f.write_str(&self.0 as &str)
			}
		}

		impl Add<&$borrowed> for $name {
			type Output = $name;

			fn add(self, rhs: &$borrowed) -> Self::Output {
				// SAFETY: for Name, NCName and CData, a concatenation with
				// strings of the same type is always also of the same type.
				// (NB: A subslice might not be, because e.g. Name has
				// constraints about what might occur in the first codepoint).
				unsafe { Self::from_native_unchecked(self.0 + &rhs.0) }
			}
		}
	}
}

macro_rules! rxml_custom_str_type {
	(
		$(#[$outer:meta])*
		pub struct $name:ident(str) use $check:ident => $owned:ident;
	) => {
		$(#[$outer])*
		#[derive(Debug, Hash, PartialEq, Eq, Ord)]
		#[repr(transparent)]
		pub struct $name(str);

		impl $name {
			#[doc = rxml_safe_str_construct_doc!($name, str, "")]
			pub fn from_str<'x>(s: &'x str) -> Result<&'x Self, WFError> {
				s.try_into()
			}

			#[doc = rxml_unsafe_str_construct_doc!($name, str)]
			pub unsafe fn from_str_unchecked<'x>(s: &'x str) -> &'x Self {
				std::mem::transmute(s)
			}
		}

		impl Deref for $name {
			type Target = str;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		impl AsRef<str> for $name {
			fn as_ref(&self) -> &str {
				&self.0
			}
		}

		impl AsRef<$name> for &$name {
			fn as_ref(&self) -> &$name {
				&self
			}
		}

		impl PartialEq<str> for $name {
			fn eq(&self, other: &str) -> bool {
				&self.0 == other
			}
		}

		impl PartialEq<$name> for str {
			fn eq(&self, other: &$name) -> bool {
				self == &other.0
			}
		}

		impl PartialOrd<$name> for $name {
			fn partial_cmp(&self, other: &$name) -> Option<Ordering> {
				self.0.partial_cmp(&other.0)
			}
		}

		impl ToOwned for $name {
			type Owned = $owned;

			fn to_owned(&self) ->Self::Owned {
				self.into()
			}
		}

		impl From<&$name> for $owned {
			fn from(other: &$name) -> Self {
				// SAFETY: $owned is assumed to use the same check; this is
				// enforced by using the pair macro.
				unsafe { $owned::from_str_unchecked(&other.0) }
			}
		}

		impl<'x> TryFrom<&'x str> for &'x $name {
			type Error = WFError;

			fn try_from(other: &'x str) -> Result<Self, Self::Error> {
				$check(other)?;
				// SAFETY: the content check is executed right above and we're
				// transmuting &str into a repr(transparent) of &str.
				Ok(unsafe { std::mem::transmute(other) } )
			}
		}

		impl fmt::Display for $name {
			fn fmt<'f>(&self, f: &'f mut fmt::Formatter) -> fmt::Result {
				f.write_str(&self.0)
			}
		}
	}
}

macro_rules! rxml_custom_string_type_pair {
	(
		$(#[$ownedmeta:meta])*
		pub struct $owned:ident($string:ty) use $check:ident;

		$(#[$borrowedmeta:meta])*
		pub struct $borrowed:ident(str);
	) => {
		rxml_custom_string_type!{
			$(#[$ownedmeta])*
			pub struct $owned($string) use $check => $borrowed;
		}

		rxml_custom_str_type!{
			$(#[$borrowedmeta])*
			pub struct $borrowed(str) use $check => $owned;
		}
	}
}

rxml_custom_string_type_pair! {
	/// String which conforms to the Name production of XML 1.0.
	///
	/// [`Name`] corresponds to a (restricted) [`String`]. For a [`str`]-like type
	/// with the same restrictions, see [`NameStr`]. `&NameStr` can be created
	/// from a string literal at compile time using the `xml_name` macro from
	/// [`rxml_proc`](https://docs.rs/rxml_proc).
	///
	/// Since [`Name`] derefs to [`String`], all (non-mutable) methods from
	/// [`String`] are available.
	///
	/// # Formal definition
	///
	/// The data inside [`Name`] (and [`NameStr`]) is guaranteed to conform to
	/// the `Name` production of the below grammar, quoted from
	/// [XML 1.0 § 2.3](https://www.w3.org/TR/REC-xml/#NT-NameStartChar):
	///
	/// ```text
	/// [4]  NameStartChar ::= ":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6]
	///                        | [#xD8-#xF6] | [#xF8-#x2FF] | [#x370-#x37D]
	///                        | [#x37F-#x1FFF] | [#x200C-#x200D]
	///                        | [#x2070-#x218F] | [#x2C00-#x2FEF]
	///                        | [#x3001-#xD7FF] | [#xF900-#xFDCF]
	///                        | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]
	/// [4a] NameChar      ::= NameStartChar | "-" | "." | [0-9] | #xB7
	///                        | [#x0300-#x036F] | [#x203F-#x2040]
	/// [5]  Name          ::= NameStartChar (NameChar)*
	/// ```
	pub struct Name(SmartString) use validate_name;

	/// str which conforms to the Name production of XML 1.0.
	///
	/// [`NameStr`] corresponds to a (restricted) [`str`]. For a [`String`]-like
	/// type with the same restrictions as well as the formal definition of those
	/// restrictions, see [`Name`].
	///
	/// `&NameStr` can be created from a string literal at compile time using the
	/// `xml_name` macro from [`rxml_proc`](https://docs.rs/rxml_proc).
	///
	/// Since [`NameStr`] derefs to [`str`], all (non-mutable) methods from
	/// [`str`] are available.
	pub struct NameStr(str);
}

impl Name {
	/// Split the name at a colon, if it exists.
	///
	/// If the name contains no colon, the function returns `(None, self)`.
	/// If the name contains exactly one colon, the function returns the part
	/// before the colon (the prefix) in the first return value and the part
	/// following the colon (the suffix) as second return value.
	///
	/// If neither of the two cases apply or the string on either side of the
	/// colon is empty, an error is returned.
	pub fn split_name(self) -> Result<(Option<NCName>, NCName), NWFError> {
		let mut name = self.0;
		let colon_pos = match name.find(':') {
			None => return Ok((None, unsafe { NCName::from_smartstring_unchecked(name) })),
			Some(pos) => pos,
		};
		if colon_pos == 0 || colon_pos == name.len() - 1 {
			return Err(NWFError::EmptyNamePart(ERRCTX_UNKNOWN));
		}

		let localname = name.split_off(colon_pos + 1);
		let mut prefix = name;

		if localname.find(':').is_some() {
			// Namespaces in XML 1.0 (Third Edition) namespace-well-formed criterium 1
			return Err(NWFError::MultiColonName(ERRCTX_UNKNOWN));
		};
		if !selectors::CLASS_XML_NAMESTART.select(localname.chars().next().unwrap()) {
			// Namespaces in XML 1.0 (Third Edition) NCName production
			return Err(NWFError::InvalidLocalName(ERRCTX_UNKNOWN));
		}

		prefix.pop();
		// do not shrink to fit here -- the prefix will be used when the element
		// is finalized to put it on the stack for quick validation of the
		// </element> token.

		debug_assert!(prefix.len() > 0);
		debug_assert!(localname.len() > 0);
		Ok((
			Some(unsafe { NCName::from_smartstring_unchecked(prefix) }),
			unsafe { NCName::from_smartstring_unchecked(localname) },
		))
	}
}

impl NameStr {
	pub fn to_name(&self) -> Name {
		self.into()
	}
}

impl From<NCName> for Name {
	fn from(other: NCName) -> Self {
		other.as_name()
	}
}

impl<'x> From<&'x NCNameStr> for &'x NameStr {
	fn from(other: &'x NCNameStr) -> Self {
		other.as_namestr()
	}
}

rxml_custom_string_type_pair! {
	/// String which conforms to the NCName production of Namespaces in XML 1.0.
	///
	/// [`NCName`] corresponds to a (restricted) [`String`]. For a [`str`]-like
	/// type with the same restrictions, see [`NCNameStr`]. `&NCNameStr` can be
	/// created from a string literal at compile time using the `xml_ncname` macro
	/// from [`rxml_proc`](https://docs.rs/rxml_proc).
	///
	/// Since [`NCName`] derefs to [`String`], all (non-mutable) methods from
	/// [`String`] are available.
	///
	/// # Formal definition
	///
	/// The data inside [`NCName`] (and [`NCNameStr`]) is guaranteed to conform to
	/// the `NCName` production of the below grammar, quoted from
	/// [Namespaces in XML 1.0 § 3](https://www.w3.org/TR/REC-xml-names/#NT-NCName):
	///
	/// ```text
	/// [4] NCName ::= Name - (Char* ':' Char*)  /* An XML Name, minus the ":" */
	/// ```
	pub struct NCName(SmartString) use validate_ncname;

	/// str which conforms to the NCName production of Namespaces in XML 1.0.
	///
	/// [`NCNameStr`] corresponds to a (restricted) [`str`]. For a [`String`]-like
	/// type with the same restrictions as well as the formal definition of those
	/// restrictions, see [`NCName`].
	///
	/// `&NCNameStr` can be created from a string literal at compile time using
	/// the `xml_ncname` macro from [`rxml_proc`](https://docs.rs/rxml_proc).
	///
	/// Since [`NCNameStr`] derefs to [`str`], all (non-mutable) methods from
	/// [`str`] are available.
	pub struct NCNameStr(str);
}

impl NCName {
	/// Compose two [`NCName`] objects to one [`Name`], separating them with
	/// a colon.
	///
	/// As an [`NCName`] is always a valid [`Name`], the composition of the
	/// two with a `:` as separator is also a valid [`Name`].
	///
	/// This is the inverse of [`Name::split_name()`].
	///
	/// # Example
	///
	/// ```
	/// # use rxml::NCName;
	/// let prefix = NCName::from_str("xmlns").unwrap();
	/// let localname = NCName::from_str("stream").unwrap();
	/// assert_eq!(prefix.add_suffix(&localname), "xmlns:stream");
	/// ```
	pub fn add_suffix(self, suffix: &NCNameStr) -> Name {
		let mut s: String = self.0.into();
		s.reserve(suffix.len() + 1);
		s.push_str(":");
		s.push_str(suffix);
		// SAFETY: NCName cannot contain a colon; Name is NCName with colons,
		// so we can concat two NCNames to a Name.
		unsafe { Name::from_string_unchecked(s) }
	}

	pub fn as_name(self) -> Name {
		// SAFETY: NCName is a strict subset of Name
		unsafe { Name::from_smartstring_unchecked(self.0) }
	}
}

impl NCNameStr {
	pub fn to_ncname(&self) -> NCName {
		self.into()
	}

	pub fn to_name(&self) -> Name {
		self.to_ncname().as_name()
	}

	pub fn as_namestr<'x>(&'x self) -> &'x NameStr {
		// SAFETY: NCName is a strict subset of Name
		unsafe { NameStr::from_str_unchecked(&self.0) }
	}

	/// Compose two [`NCName`] objects to one [`Name`], separating them with
	/// a colon.
	///
	/// As an [`NCName`] is always a valid [`Name`], the composition of the
	/// two with a `:` as separator is also a valid [`Name`].
	///
	/// This is the inverse of [`Name::split_name()`].
	///
	/// # Example
	///
	/// ```
	/// # use rxml::NCName;
	/// let prefix = NCName::from_str("xmlns").unwrap();
	/// let localname = NCName::from_str("stream").unwrap();
	/// assert_eq!(prefix.add_suffix(&localname), "xmlns:stream");
	/// ```
	pub fn with_suffix(&self, suffix: &NCNameStr) -> Name {
		let mut s = String::with_capacity(self.len() + 1 + suffix.len());
		s.push_str(self);
		s.push_str(":");
		s.push_str(suffix);
		// SAFETY: NCName cannot contain a colon; Name is NCName with colons,
		// so we can concat two NCNames to a Name.
		unsafe { Name::from_string_unchecked(s) }
	}
}

rxml_custom_string_type_pair! {
	/// String which consists only of XML 1.0 Chars.
	///
	/// [`CData`] corresponds to a (restricted) [`String`]. For a [`str`]-like
	/// type with the same restrictions, see [`CDataStr`]. `&CDataStr` can be
	/// created from a string literal at compile time using the `xml_cdata` macro
	/// from [`rxml_proc`](https://docs.rs/rxml_proc).
	///
	/// Since [`CData`] derefs to [`String`], all (non-mutable) methods from
	/// [`String`] are available.
	///
	/// # Formal definition
	///
	/// The data inside [`CData`] (and [`CDataStr`]) is guaranteed to be a
	/// sequence of `Char` as defined in
	/// [XML 1.0 § 2.2](https://www.w3.org/TR/REC-xml/#NT-Char) and quoted below:
	///
	/// ```text
	/// [2] Char ::= #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD]
	///              | [#x10000-#x10FFFF]
	///              /* any Unicode character, excluding the surrogate blocks,
	///                 FFFE, and FFFF. */
	/// ```
	///
	/// This is a Unicode scalar value, minus ASCII control characters except
	/// Tab (`\x09`), CR (`\x0d`) and LF (`\x0a`), the BOM (`\u{fffe}`) and
	/// whatever `\u{ffff}` is.
	///
	/// # Escaping
	///
	/// [`CData`] objects do not contain references or CDATA sections as those are
	/// expanded by the lexer. This implies that `CData` objects are not safe to
	/// just verbatimly copy into an XML document; additional escaping may be
	/// necessary.
	pub struct CData(String) use validate_cdata;

	/// str which consists only of XML 1.0 Chars.
	///
	/// [`CDataStr`] corresponds to a (restricted) [`str`]. For a [`String`]-like
	/// type with the same restrictions as well as the formal definition of those
	/// restrictions, see [`CData`].
	///
	/// `&CDataStr` can be created from a string literal at compile time using the
	/// `xml_cdata` macro from [`rxml_proc`](https://docs.rs/rxml_proc).
	///
	/// Since [`CDataStr`] derefs to [`str`], all (non-mutable) methods from
	/// [`str`] are available.
	pub struct CDataStr(str);
}

impl CDataStr {
	pub fn to_cdata(&self) -> CData {
		self.into()
	}
}

impl From<NCName> for CData {
	fn from(other: NCName) -> Self {
		// SAFETY: NCNames can only consist of valid XML 1.0 chars, so they
		// are also valid CData
		unsafe { CData::from_smartstring_unchecked(other.0) }
	}
}

impl From<Name> for CData {
	fn from(other: Name) -> Self {
		// SAFETY: Names can only consist of valid XML 1.0 chars, so they
		// are also valid CData
		unsafe { CData::from_smartstring_unchecked(other.0) }
	}
}

impl<'x> From<&'x NCNameStr> for &'x CDataStr {
	fn from(other: &'x NCNameStr) -> Self {
		// SAFETY: NCNames can only consist of valid XML 1.0 chars, so they
		// are also valid CData
		unsafe { CDataStr::from_str_unchecked(&other.0) }
	}
}

impl<'x> From<&'x NameStr> for &'x CDataStr {
	fn from(other: &'x NameStr) -> Self {
		// SAFETY: Names can only consist of valid XML 1.0 chars, so they
		// are also valid CData
		unsafe { CDataStr::from_str_unchecked(&other.0) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn split_name_rejects_localname_with_non_namestart_first_char() {
		let nm: Name = "foo:-bar".try_into().unwrap();
		let result = nm.split_name();
		assert!(matches!(
			result.err().unwrap(),
			NWFError::InvalidLocalName(_)
		));
	}

	#[test]
	fn cdatastr_allows_slashes() {
		let _: &CDataStr = "http://www.w3.org/XML/1998/namespace".try_into().unwrap();
	}
}
