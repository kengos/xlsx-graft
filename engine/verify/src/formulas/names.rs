//! [`Names`]: the names a workbook defines, which a formula's words are
//! looked up in.

use std::collections::{BTreeMap, BTreeSet};

/// The workbook's defined names and table names.
///
/// Names are compared as a formula's are, upper-cased in ASCII: one that
/// differs from a formula's only in the case of a letter outside ASCII is
/// another name, and the formula's word is denied as a function the table
/// does not know. Two defined names of one name in one scope, which Excel
/// would not save, are both kept: a formula naming it is judged by both.
///
/// # Examples
///
/// ```
/// use xlsx_graft_verify::Names;
///
/// let names = Names::default()
///     .defined_name("MyLambda", None, "_xlfn.LAMBDA(_xlpm.x,_xlpm.x*2)")
///     .defined_name("Rate", Some(1), "Sheet2!$B$1")
///     .table("Table1");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    /// The defined names, in the order given.
    defined: Vec<DefinedName>,
    /// The buckets of the defined names of each upper-cased name.
    groups: BTreeMap<String, Group>,
    /// Each bucket: a set of defined names a word may mean, as indexes
    /// into `defined`.
    buckets: Vec<Vec<usize>>,
    /// The table names, upper-cased.
    tables: BTreeSet<String>,
}

/// One defined name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DefinedName {
    /// The name as written.
    pub(super) name: String,
    /// The `localSheetId` of the sheet it is scoped to; `None` for the
    /// workbook.
    pub(super) sheet: Option<u32>,
    /// Its formula, with no leading `=`.
    pub(super) formula: String,
}

/// The buckets of the defined names of one name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    /// The bucket of all of them.
    all: Bucket,
    /// The bucket of those of each scope: a sheet's `localSheetId`, or
    /// `None` for the workbook.
    scoped: BTreeMap<Option<u32>, Bucket>,
}

/// A set of defined names a word may mean: a name's in every scope, or in
/// one. A word is resolved to one bucket, so the bucket is the one thing
/// a caller that looks each up once keys on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Bucket(usize);

impl Bucket {
    /// Its index, below [`Names::bucket_count`].
    pub(super) fn index(self) -> usize {
        self.0
    }
}

/// Which defined names of a name a word may mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    /// Any of them.
    Any,
    /// Those of the sheet at this index, or the workbook's when it has
    /// none, or, rather than none, any of them.
    From(Option<u32>),
}

impl Names {
    /// These names, with the defined name `name`, scoped to the sheet at
    /// `sheet` (its `localSheetId`) or to the workbook, holding `formula`
    /// (with no leading `=`).
    #[must_use]
    pub fn defined_name(mut self, name: &str, sheet: Option<u32>, formula: &str) -> Self {
        let index = self.defined.len();
        let buckets = &mut self.buckets;
        let mut bucket = || {
            buckets.push(Vec::new());
            Bucket(buckets.len().saturating_sub(1))
        };
        let group = self
            .groups
            .entry(name.to_ascii_uppercase())
            .or_insert_with(|| Group {
                all: bucket(),
                scoped: BTreeMap::new(),
            });
        let scoped = *group.scoped.entry(sheet).or_insert_with(&mut bucket);
        for Bucket(at) in [group.all, scoped] {
            if let Some(members) = self.buckets.get_mut(at) {
                members.push(index);
            }
        }
        self.defined.push(DefinedName {
            name: name.to_owned(),
            sheet,
            formula: formula.to_owned(),
        });
        self
    }

    /// These names, with the table `name`.
    #[must_use]
    pub fn table(mut self, name: &str) -> Self {
        self.tables.insert(name.to_ascii_uppercase());
        self
    }

    /// The bucket of the defined names `name` (upper-cased) may mean in
    /// `scope`, and whether one of them is in its scope: not when a word
    /// whose sheet and workbook have none means those of every scope.
    /// `None` when it means none.
    pub(super) fn resolve(&self, name: &str, scope: Scope) -> Option<(Bucket, bool)> {
        let group = self.groups.get(name)?;
        match scope {
            Scope::Any => Some((group.all, true)),
            Scope::From(sheet) => sheet
                .and_then(|sheet| group.scoped.get(&Some(sheet)))
                .or_else(|| group.scoped.get(&None))
                .map(|&bucket| (bucket, true))
                .or(Some((group.all, false))),
        }
    }

    /// The defined names in `bucket`, as indexes.
    pub(super) fn members(&self, Bucket(at): Bucket) -> &[usize] {
        self.buckets.get(at).map_or(&[], Vec::as_slice)
    }

    /// How many buckets there are; each is below this.
    pub(super) fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    /// How many defined names there are; each index is below this.
    pub(super) fn len(&self) -> usize {
        self.defined.len()
    }

    /// Whether `name` (upper-cased) is a table's.
    pub(super) fn is_table(&self, name: &str) -> bool {
        self.tables.contains(name)
    }

    /// The defined name at `index`.
    pub(super) fn get(&self, index: usize) -> Option<&DefinedName> {
        self.defined.get(index)
    }

    /// The name of the defined name at `index`, as written.
    pub(super) fn name(&self, index: usize) -> &str {
        self.get(index).map_or("", |defined| defined.name.as_str())
    }
}
