use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::document;
use crate::version::{self, Version};
use crate::{Auditfile, AuditfileStream, Error, v4};

impl Auditfile {
    /// Reads an auditfile from a reader.
    ///
    /// The version is determined from the namespace of the root element before the rest of the
    /// file is interpreted, so a file of another version yields [`Error::UnsupportedVersion`]
    /// rather than an error about its contents. The whole file is read into memory.
    ///
    /// ```
    /// use xaf_auditfile::Auditfile;
    ///
    /// let xml = r#"
    /// <auditfile xmlns="http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0">
    ///   <header>
    ///     <fiscalYear>2024</fiscalYear>
    ///     <startDate>2024-01-01</startDate>
    ///     <endDate>2024-12-31</endDate>
    ///     <curCode>EUR</curCode>
    ///     <dateCreated>2025-02-01</dateCreated>
    ///     <softwareDesc>Example</softwareDesc>
    ///     <softwareVersion>1.0</softwareVersion>
    ///   </header>
    ///   <company>
    ///     <companyName>Example B.V.</companyName>
    ///     <taxRegistrationCountry>NL</taxRegistrationCountry>
    ///     <taxRegIdent>NL000000000B01</taxRegIdent>
    ///   </company>
    /// </auditfile>"#;
    ///
    /// let file = Auditfile::from_reader(xml.as_bytes())?;
    /// assert_eq!(file.company.name, "Example B.V.");
    /// # Ok::<(), xaf_auditfile::Error>(())
    /// ```
    pub fn from_reader<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }

    /// Reads an auditfile from a file. See [`Auditfile::from_reader`].
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::from_bytes(&fs::read(path)?)
    }

    /// Reads an auditfile transaction by transaction instead of as a whole.
    ///
    /// **This is an exception, not the way to read a file.** [`Auditfile::from_reader`] and
    /// [`Auditfile::from_path`] are the normal route: they give the complete model, accept
    /// elements in any order and are what [`Auditfile::validate`] works on. Use streaming only for
    /// a file so large that it does not fit in memory, where the memory use of reading it whole
    /// (roughly 1.5 to 2 KB per transaction line) is the problem.
    ///
    /// What streaming changes:
    ///
    /// - [`AuditfileStream::auditfile`] holds everything before the first journal, with `journals`
    ///   empty. The transactions follow one at a time, in document order.
    /// - The elements must be in the order of the schema, with the transactions last. A file that
    ///   does not keep to it fails with [`Error::OutOfOrder`] where a whole-file read would
    ///   accept it.
    /// - Journals without transactions are not yielded.
    /// - [`Auditfile::validate`] needs the whole model. Use [`Auditfile::validate_master_data`]
    ///   and a [`Validator`](crate::Validator) instead; together they give the same findings.
    ///
    /// The version and the master data are checked up front, so an unsupported version or an
    /// unusable header fails here and not at the first transaction. Errors in a transaction
    /// surface when that transaction is reached, after which the stream ends.
    pub fn stream_from_reader<R: BufRead>(reader: R) -> Result<AuditfileStream<R>, Error> {
        v4::stream(reader)
    }

    /// Streams an auditfile from a file. See [`Auditfile::stream_from_reader`]: this too is for
    /// exceptionally large files only.
    ///
    /// ```no_run
    /// use xaf_auditfile::Auditfile;
    ///
    /// let stream = Auditfile::stream_from_path("huge.xaf")?;
    ///
    /// // Header, company, master data and opening balance; `journals` is empty.
    /// let mut findings = stream.auditfile().validate_master_data();
    /// let mut validator = stream.auditfile().validator();
    ///
    /// for item in stream {
    ///     let item = item?;
    ///     findings.extend(validator.check(&item.journal, &item.transaction));
    ///     // handle item.transaction here
    /// }
    /// findings.extend(validator.finish());
    /// # Ok::<(), xaf_auditfile::Error>(())
    /// ```
    pub fn stream_from_path(
        path: impl AsRef<Path>,
    ) -> Result<AuditfileStream<BufReader<File>>, Error> {
        Self::stream_from_reader(BufReader::new(File::open(path)?))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        match version::detect(bytes)? {
            Version::V4 => {
                document::check(bytes)?;
                v4::parse(bytes)
            }
        }
    }
}
