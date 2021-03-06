// This is a part of rust-chrono.
// Copyright (c) 2014-2015, Kang Seonghoon.
// See README.md and LICENSE.txt for details.

/*!
 * ISO 8601 date and time with time zone.
 */

use std::{str, fmt, hash};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

use {Weekday, Timelike, Datelike};
use offset::{TimeZone, Offset};
use offset::utc::UTC;
use offset::local::Local;
use offset::fixed::FixedOffset;
use duration::Duration;
use naive::time::NaiveTime;
use naive::datetime::NaiveDateTime;
use date::Date;
use format::{Item, Numeric, Pad, Fixed};
use format::{parse, Parsed, ParseError, ParseResult, DelayedFormat, StrftimeItems};

/// ISO 8601 combined date and time with time zone.
#[derive(Clone)]
pub struct DateTime<Tz: TimeZone> {
    datetime: NaiveDateTime,
    offset: Tz::Offset,
}

impl<Tz: TimeZone> DateTime<Tz> {
    /// Makes a new `DateTime` with given *UTC* datetime and offset.
    /// The local datetime should be constructed via the `TimeZone` trait.
    //
    // note: this constructor is purposedly not named to `new` to discourage the direct usage.
    #[inline]
    pub fn from_utc(datetime: NaiveDateTime, offset: Tz::Offset) -> DateTime<Tz> {
        DateTime { datetime: datetime, offset: offset }
    }

    /// Retrieves a date component.
    #[inline]
    pub fn date(&self) -> Date<Tz> {
        Date::from_utc(self.datetime.date().clone(), self.offset.clone())
    }

    /// Retrieves a time component.
    /// Unlike `date`, this is not associated to the time zone.
    #[inline]
    pub fn time(&self) -> NaiveTime {
        self.datetime.time() + self.offset.local_minus_utc()
    }

    /// Returns the number of non-leap seconds since January 1, 1970 0:00:00 UTC
    /// (aka "UNIX timestamp").
    #[inline]
    pub fn timestamp(&self) -> i64 {
        self.datetime.timestamp()
    }

    /// Same to `DateTime::timestamp`.
    #[inline]
    #[deprecated = "Use `DateTime::timestamp` instead."]
    pub fn num_seconds_from_unix_epoch(&self) -> i64 {
        self.timestamp()
    }

    /// Retrieves an associated offset from UTC.
    #[inline]
    pub fn offset<'a>(&'a self) -> &'a Tz::Offset {
        &self.offset
    }

    /// Retrieves an associated time zone.
    #[inline]
    pub fn timezone(&self) -> Tz {
        TimeZone::from_offset(&self.offset)
    }

    /// Changes the associated time zone.
    /// This does not change the actual `DateTime` (but will change the string representation).
    #[inline]
    pub fn with_timezone<Tz2: TimeZone>(&self, tz: &Tz2) -> DateTime<Tz2> {
        tz.from_utc_datetime(&self.datetime)
    }

    /// Adds given `Duration` to the current date and time.
    ///
    /// Returns `None` when it will result in overflow.
    #[inline]
    pub fn checked_add(self, rhs: Duration) -> Option<DateTime<Tz>> {
        let datetime = try_opt!(self.datetime.checked_add(rhs));
        Some(DateTime { datetime: datetime, offset: self.offset })
    }

    /// Subtracts given `Duration` from the current date and time.
    ///
    /// Returns `None` when it will result in overflow.
    #[inline]
    pub fn checked_sub(self, rhs: Duration) -> Option<DateTime<Tz>> {
        let datetime = try_opt!(self.datetime.checked_sub(rhs));
        Some(DateTime { datetime: datetime, offset: self.offset })
    }

    /// Returns a view to the naive UTC datetime.
    #[inline]
    pub fn naive_utc(&self) -> NaiveDateTime {
        self.datetime
    }

    /// Returns a view to the naive local datetime.
    #[inline]
    pub fn naive_local(&self) -> NaiveDateTime {
        self.datetime + self.offset.local_minus_utc()
    }
}

/// Maps the local datetime to other datetime with given conversion function.
fn map_local<Tz: TimeZone, F>(dt: &DateTime<Tz>, mut f: F) -> Option<DateTime<Tz>>
        where F: FnMut(NaiveDateTime) -> Option<NaiveDateTime> {
    f(dt.naive_local()).and_then(|datetime| dt.timezone().from_local_datetime(&datetime).single())
}

impl DateTime<FixedOffset> {
    /// Parses an RFC 2822 date and time string such as `Tue, 1 Jul 2003 10:52:37 +0200`,
    /// then returns a new `DateTime` with a parsed `FixedOffset`.
    pub fn parse_from_rfc2822(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        const ITEMS: &'static [Item<'static>] = &[Item::Fixed(Fixed::RFC2822)];
        let mut parsed = Parsed::new();
        try!(parse(&mut parsed, s, ITEMS.iter().cloned()));
        parsed.to_datetime()
    }

    /// Parses an RFC 3339 and ISO 8601 date and time string such as `1996-12-19T16:39:57-08:00`,
    /// then returns a new `DateTime` with a parsed `FixedOffset`.
    ///
    /// Why isn't this named `parse_from_iso8601`? That's because ISO 8601 allows some freedom
    /// over the syntax and RFC 3339 exercises that freedom to rigidly define a fixed format.
    pub fn parse_from_rfc3339(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        const ITEMS: &'static [Item<'static>] = &[Item::Fixed(Fixed::RFC3339)];
        let mut parsed = Parsed::new();
        try!(parse(&mut parsed, s, ITEMS.iter().cloned()));
        parsed.to_datetime()
    }

    /// Parses a string with the specified format string and
    /// returns a new `DateTime` with a parsed `FixedOffset`.
    /// See the `format::strftime` module on the supported escape sequences.
    ///
    /// See also `Offset::datetime_from_str` which gives a local `DateTime` on specific time zone.
    pub fn parse_from_str(s: &str, fmt: &str) -> ParseResult<DateTime<FixedOffset>> {
        let mut parsed = Parsed::new();
        try!(parse(&mut parsed, s, StrftimeItems::new(fmt)));
        parsed.to_datetime()
    }
}

impl<Tz: TimeZone> DateTime<Tz> where Tz::Offset: fmt::Display {
    /// Returns an RFC 2822 date and time string such as `Tue, 1 Jul 2003 10:52:37 +0200`.
    pub fn to_rfc2822(&self) -> String {
        const ITEMS: &'static [Item<'static>] = &[Item::Fixed(Fixed::RFC2822)];
        self.format_with_items(ITEMS.iter().cloned()).to_string()
    }

    /// Returns an RFC 3339 and ISO 8601 date and time string such as `1996-12-19T16:39:57-08:00`.
    pub fn to_rfc3339(&self) -> String {
        const ITEMS: &'static [Item<'static>] = &[Item::Fixed(Fixed::RFC3339)];
        self.format_with_items(ITEMS.iter().cloned()).to_string()
    }

    /// Formats the combined date and time with the specified formatting items.
    #[inline]
    pub fn format_with_items<'a, I>(&'a self, items: I) -> DelayedFormat<'a, I>
            where I: Iterator<Item=Item<'a>> + Clone {
        let local = self.naive_local();
        DelayedFormat::new_with_offset(Some(local.date()), Some(local.time()), &self.offset, items)
    }

    /// Formats the combined date and time with the specified format string.
    /// See the `format::strftime` module on the supported escape sequences.
    #[inline]
    pub fn format<'a>(&'a self, fmt: &'a str) -> DelayedFormat<'a, StrftimeItems<'a>> {
        self.format_with_items(StrftimeItems::new(fmt))
    }
}

impl<Tz: TimeZone> Datelike for DateTime<Tz> {
    #[inline] fn year(&self) -> i32 { self.naive_local().year() }
    #[inline] fn month(&self) -> u32 { self.naive_local().month() }
    #[inline] fn month0(&self) -> u32 { self.naive_local().month0() }
    #[inline] fn day(&self) -> u32 { self.naive_local().day() }
    #[inline] fn day0(&self) -> u32 { self.naive_local().day0() }
    #[inline] fn ordinal(&self) -> u32 { self.naive_local().ordinal() }
    #[inline] fn ordinal0(&self) -> u32 { self.naive_local().ordinal0() }
    #[inline] fn weekday(&self) -> Weekday { self.naive_local().weekday() }
    #[inline] fn isoweekdate(&self) -> (i32, u32, Weekday) { self.naive_local().isoweekdate() }

    #[inline]
    fn with_year(&self, year: i32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_year(year))
    }

    #[inline]
    fn with_month(&self, month: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_month(month))
    }

    #[inline]
    fn with_month0(&self, month0: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_month0(month0))
    }

    #[inline]
    fn with_day(&self, day: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_day(day))
    }

    #[inline]
    fn with_day0(&self, day0: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_day0(day0))
    }

    #[inline]
    fn with_ordinal(&self, ordinal: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_ordinal(ordinal))
    }

    #[inline]
    fn with_ordinal0(&self, ordinal0: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_ordinal0(ordinal0))
    }
}

impl<Tz: TimeZone> Timelike for DateTime<Tz> {
    #[inline] fn hour(&self) -> u32 { self.naive_local().hour() }
    #[inline] fn minute(&self) -> u32 { self.naive_local().minute() }
    #[inline] fn second(&self) -> u32 { self.naive_local().second() }
    #[inline] fn nanosecond(&self) -> u32 { self.naive_local().nanosecond() }

    #[inline]
    fn with_hour(&self, hour: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_hour(hour))
    }

    #[inline]
    fn with_minute(&self, min: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_minute(min))
    }

    #[inline]
    fn with_second(&self, sec: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_second(sec))
    }

    #[inline]
    fn with_nanosecond(&self, nano: u32) -> Option<DateTime<Tz>> {
        map_local(self, |datetime| datetime.with_nanosecond(nano))
    }
}

impl<Tz: TimeZone, Tz2: TimeZone> PartialEq<DateTime<Tz2>> for DateTime<Tz> {
    fn eq(&self, other: &DateTime<Tz2>) -> bool { self.datetime == other.datetime }
}

impl<Tz: TimeZone> Eq for DateTime<Tz> {
}

impl<Tz: TimeZone> PartialOrd for DateTime<Tz> {
    fn partial_cmp(&self, other: &DateTime<Tz>) -> Option<Ordering> {
        self.datetime.partial_cmp(&other.datetime)
    }
}

impl<Tz: TimeZone> Ord for DateTime<Tz> {
    fn cmp(&self, other: &DateTime<Tz>) -> Ordering { self.datetime.cmp(&other.datetime) }
}

impl<Tz: TimeZone, H: hash::Hasher + hash::Writer> hash::Hash<H> for DateTime<Tz> {
    fn hash(&self, state: &mut H) { self.datetime.hash(state) }
}

impl<Tz: TimeZone> Add<Duration> for DateTime<Tz> {
    type Output = DateTime<Tz>;

    #[inline]
    fn add(self, rhs: Duration) -> DateTime<Tz> {
        self.checked_add(rhs).expect("`DateTime + Duration` overflowed")
    }
}

impl<Tz: TimeZone, Tz2: TimeZone> Sub<DateTime<Tz2>> for DateTime<Tz> {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: DateTime<Tz2>) -> Duration { self.datetime - rhs.datetime }
}

impl<Tz: TimeZone> Sub<Duration> for DateTime<Tz> {
    type Output = DateTime<Tz>;

    #[inline]
    fn sub(self, rhs: Duration) -> DateTime<Tz> {
        self.checked_sub(rhs).expect("`DateTime - Duration` overflowed")
    }
}

impl<Tz: TimeZone> fmt::Debug for DateTime<Tz> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}{:?}", self.naive_local(), self.offset)
    }
}

impl<Tz: TimeZone> fmt::Display for DateTime<Tz> where Tz::Offset: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.naive_local(), self.offset)
    }
}

impl str::FromStr for DateTime<FixedOffset> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        const ITEMS: &'static [Item<'static>] = &[
            Item::Space(""), Item::Numeric(Numeric::Year, Pad::Zero),
            Item::Space(""), Item::Literal("-"),
            Item::Space(""), Item::Numeric(Numeric::Month, Pad::Zero),
            Item::Space(""), Item::Literal("-"),
            Item::Space(""), Item::Numeric(Numeric::Day, Pad::Zero),
            Item::Space(""), Item::Literal("T"), // XXX shouldn't this be case-insensitive?
            Item::Space(""), Item::Numeric(Numeric::Hour, Pad::Zero),
            Item::Space(""), Item::Literal(":"),
            Item::Space(""), Item::Numeric(Numeric::Minute, Pad::Zero),
            Item::Space(""), Item::Literal(":"),
            Item::Space(""), Item::Numeric(Numeric::Second, Pad::Zero),
                             Item::Fixed(Fixed::Nanosecond),
            Item::Space(""), Item::Fixed(Fixed::TimezoneOffsetZ),
            Item::Space(""),
        ];

        let mut parsed = Parsed::new();
        try!(parse(&mut parsed, s, ITEMS.iter().cloned()));
        parsed.to_datetime()
    }
}

impl str::FromStr for DateTime<UTC> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<UTC>> {
        s.parse::<DateTime<FixedOffset>>().map(|dt| dt.with_timezone(&UTC))
    }
}

impl str::FromStr for DateTime<Local> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<Local>> {
        s.parse::<DateTime<FixedOffset>>().map(|dt| dt.with_timezone(&Local))
    }
}

#[cfg(test)]
mod tests {
    use super::DateTime;
    use Datelike;
    use naive::time::NaiveTime;
    use duration::Duration;
    use offset::TimeZone;
    use offset::utc::UTC;
    use offset::local::Local;
    use offset::fixed::FixedOffset;

    #[test]
    #[allow(non_snake_case)]
    fn test_datetime_offset() {
        let EST = FixedOffset::east(5*60*60);
        let EDT = FixedOffset::east(4*60*60);

        assert_eq!(format!("{}", UTC.ymd(2014, 5, 6).and_hms(7, 8, 9)),
                   "2014-05-06 07:08:09 UTC");
        assert_eq!(format!("{}", EDT.ymd(2014, 5, 6).and_hms(7, 8, 9)),
                   "2014-05-06 07:08:09 +04:00");
        assert_eq!(format!("{:?}", UTC.ymd(2014, 5, 6).and_hms(7, 8, 9)),
                   "2014-05-06T07:08:09Z");
        assert_eq!(format!("{:?}", EDT.ymd(2014, 5, 6).and_hms(7, 8, 9)),
                   "2014-05-06T07:08:09+04:00");

        assert_eq!(UTC.ymd(2014, 5, 6).and_hms(7, 8, 9), EDT.ymd(2014, 5, 6).and_hms(11, 8, 9));
        assert_eq!(UTC.ymd(2014, 5, 6).and_hms(7, 8, 9) + Duration::seconds(3600 + 60 + 1),
                   UTC.ymd(2014, 5, 6).and_hms(8, 9, 10));
        assert_eq!(UTC.ymd(2014, 5, 6).and_hms(7, 8, 9) - EDT.ymd(2014, 5, 6).and_hms(10, 11, 12),
                   Duration::seconds(3600 - 3*60 - 3));

        assert_eq!(*UTC.ymd(2014, 5, 6).and_hms(7, 8, 9).offset(), UTC);
        assert_eq!(*EDT.ymd(2014, 5, 6).and_hms(7, 8, 9).offset(), EDT);
        assert!(*EDT.ymd(2014, 5, 6).and_hms(7, 8, 9).offset() != EST);
    }

    #[test]
    fn test_datetime_time() {
        assert_eq!(FixedOffset::east(5*60*60).ymd(2014, 5, 6).and_hms(7, 8, 9).time(),
                   NaiveTime::from_hms(7, 8, 9));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_datetime_rfc2822_and_rfc3339() {
        let EDT = FixedOffset::east(5*60*60);
        assert_eq!(UTC.ymd(2015, 2, 18).and_hms(23, 16, 9).to_rfc2822(),
                   "Wed, 18 Feb 2015 23:16:09 +0000");
        assert_eq!(UTC.ymd(2015, 2, 18).and_hms(23, 16, 9).to_rfc3339(),
                   "2015-02-18T23:16:09+00:00");
        assert_eq!(EDT.ymd(2015, 2, 18).and_hms_milli(23, 16, 9, 150).to_rfc2822(),
                   "Wed, 18 Feb 2015 23:16:09 +0500");
        assert_eq!(EDT.ymd(2015, 2, 18).and_hms_milli(23, 16, 9, 150).to_rfc3339(),
                   "2015-02-18T23:16:09.150+05:00");
        assert_eq!(EDT.ymd(2015, 2, 18).and_hms_micro(23, 59, 59, 1_234_567).to_rfc2822(),
                   "Wed, 18 Feb 2015 23:59:60 +0500");
        assert_eq!(EDT.ymd(2015, 2, 18).and_hms_micro(23, 59, 59, 1_234_567).to_rfc3339(),
                   "2015-02-18T23:59:60.234567+05:00");

        assert_eq!(DateTime::parse_from_rfc2822("Wed, 18 Feb 2015 23:16:09 +0000"),
                   Ok(FixedOffset::east(0).ymd(2015, 2, 18).and_hms(23, 16, 9)));
        assert_eq!(DateTime::parse_from_rfc3339("2015-02-18T23:16:09Z"),
                   Ok(FixedOffset::east(0).ymd(2015, 2, 18).and_hms(23, 16, 9)));
        assert_eq!(DateTime::parse_from_rfc2822("Wed, 18 Feb 2015 23:59:60 +0500"),
                   Ok(EDT.ymd(2015, 2, 18).and_hms_milli(23, 59, 59, 1_000)));
        assert_eq!(DateTime::parse_from_rfc3339("2015-02-18T23:59:60.234567+05:00"),
                   Ok(EDT.ymd(2015, 2, 18).and_hms_micro(23, 59, 59, 1_234_567)));
    }

    #[test]
    fn test_datetime_from_str() {
        assert_eq!("2015-2-18T23:16:9.15Z".parse::<DateTime<FixedOffset>>(),
                   Ok(FixedOffset::east(0).ymd(2015, 2, 18).and_hms_milli(23, 16, 9, 150)));
        assert_eq!("2015-2-18T13:16:9.15-10:00".parse::<DateTime<FixedOffset>>(),
                   Ok(FixedOffset::west(10 * 3600).ymd(2015, 2, 18).and_hms_milli(13, 16, 9, 150)));
        assert!("2015-2-18T23:16:9.15".parse::<DateTime<FixedOffset>>().is_err());

        assert_eq!("2015-2-18T23:16:9.15Z".parse::<DateTime<UTC>>(),
                   Ok(UTC.ymd(2015, 2, 18).and_hms_milli(23, 16, 9, 150)));
        assert_eq!("2015-2-18T13:16:9.15-10:00".parse::<DateTime<UTC>>(),
                   Ok(UTC.ymd(2015, 2, 18).and_hms_milli(23, 16, 9, 150)));
        assert!("2015-2-18T23:16:9.15".parse::<DateTime<UTC>>().is_err());

        // no test for `DateTime<Local>`, we cannot verify that much.
    }

    #[test]
    fn test_datetime_parse_from_str() {
        let ymdhms = |&: y,m,d,h,n,s,off| FixedOffset::east(off).ymd(y,m,d).and_hms(h,n,s);
        assert_eq!(DateTime::parse_from_str("2014-5-7T12:34:56+09:30", "%Y-%m-%dT%H:%M:%S%z"),
                   Ok(ymdhms(2014, 5, 7, 12, 34, 56, 570*60))); // ignore offset
        assert!(DateTime::parse_from_str("20140507000000", "%Y%m%d%H%M%S").is_err()); // no offset
        assert!(DateTime::parse_from_str("Fri, 09 Aug 2013 23:54:35 GMT",
                                         "%a, %d %b %Y %H:%M:%S GMT").is_err());
        assert_eq!(UTC.datetime_from_str("Fri, 09 Aug 2013 23:54:35 GMT",
                                         "%a, %d %b %Y %H:%M:%S GMT"),
                   Ok(UTC.ymd(2013, 8, 9).and_hms(23, 54, 35)));
    }

    #[test]
    fn test_datetime_format_with_local() {
        // if we are not around the year boundary, local and UTC date should have the same year
        let dt = Local::now().with_month(5).unwrap();
        assert_eq!(dt.format("%Y").to_string(), dt.with_timezone(&UTC).format("%Y").to_string());
    }
}

