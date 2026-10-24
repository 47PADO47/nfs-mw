//! Fluent catalogues layered by priority: one layer per namespace and locale of a source.

use std::fmt;

use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;

use crate::{Args, Locale, pseudo};

/// The `.ftl` files one source gives a namespace in one locale: the rewrite's built-in catalogue, or a mod's folder.
#[derive(Clone, Debug)]
pub struct Source {
    /// Whose messages these are (`core` for the rewrite, `mod:<id>` for a mod).
    pub namespace: String,
    pub locale: Locale,
    /// Who supplied the files, for messages about them (`built-in`, a mod id).
    pub origin: String,
    /// `(file name, text)` of each file.
    pub files: Vec<(String, String)>,
}

/// A file that did not parse cleanly. The rest of it, and every other file, is still used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub origin: String,
    pub file: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.origin, self.file, self.message)
    }
}

struct Layer {
    namespace: String,
    locale: Locale,
    bundle: FluentBundle<FluentResource>,
}

/// Every message source of a run, highest priority first.
#[derive(Default)]
pub struct Catalog {
    layers: Vec<Layer>,
}

impl Catalog {
    /// The catalogue of `sources`, given highest priority first. With `pseudo`, each English source is also added as the
    /// pseudo-locale ([`crate::pseudo`]).
    pub fn build(sources: &[Source], pseudo: bool) -> (Self, Vec<Problem>) {
        let mut problems = Vec::new();
        let mut layers = Vec::new();
        let english = Locale::english();
        if pseudo {
            for source in sources.iter().filter(|s| s.locale == english) {
                layers.push(layer(source, Locale::pseudo(), true, &mut problems));
            }
        }
        for source in sources {
            layers.push(layer(source, source.locale.clone(), false, &mut problems));
        }
        (Self { layers }, problems)
    }

    /// Message `id` of `namespace` formatted with `args`, from the first locale of `chain` that has it. Also says which
    /// locale answered.
    pub fn format(&self, namespace: &str, id: &str, args: &Args, chain: &[Locale]) -> Option<(String, Locale)> {
        let fluent_args = args.to_fluent();
        for locale in chain {
            for layer in self.layers.iter().filter(|l| l.namespace == namespace && &l.locale == locale) {
                let Some(pattern) = layer.bundle.get_message(id).and_then(|message| message.value()) else { continue };
                let mut errors = Vec::new();
                let text = layer.bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
                if !errors.is_empty() {
                    log::debug!("{namespace} {id} ({locale}): {errors:?}");
                }
                return Some((text.into_owned(), locale.clone()));
            }
        }
        None
    }

    /// Whether `locale` itself (not its chain) has message `id` of `namespace`.
    pub fn has(&self, namespace: &str, id: &str, locale: &Locale) -> bool {
        self.layers.iter().any(|l| l.namespace == namespace && &l.locale == locale && l.bundle.has_message(id))
    }

    /// The locales that have any message of `namespace`, in priority order, each once.
    pub fn locales(&self, namespace: &str) -> Vec<Locale> {
        let mut out: Vec<Locale> = Vec::new();
        for layer in self.layers.iter().filter(|l| l.namespace == namespace) {
            if !out.contains(&layer.locale) {
                out.push(layer.locale.clone());
            }
        }
        out
    }
}

fn layer(source: &Source, locale: Locale, pseudo: bool, problems: &mut Vec<Problem>) -> Layer {
    // Plural rules come from the source's own language; the pseudo-locale uses English's.
    let mut bundle = FluentBundle::new_concurrent(vec![source.locale.id().clone()]);
    // The game's fonts have no glyphs for the Unicode isolation marks Fluent puts around variables.
    bundle.set_use_isolating(false);
    // `NUMBER` is a built-in function only once it is added: the ordinal selectors need it.
    if let Err(e) = bundle.add_builtins() {
        problems.push(Problem { origin: source.origin.clone(), file: String::new(), message: format!("{e:?}") });
    }
    if pseudo {
        bundle.set_transform(Some(pseudo::transform));
    }
    for (file, text) in &source.files {
        let problem = |message: String| Problem { origin: source.origin.clone(), file: file.clone(), message };
        let resource = match FluentResource::try_new(text.clone()) {
            Ok(resource) => resource,
            Err((resource, errors)) => {
                problems.extend(errors.iter().map(|e| problem(format!("{e:?}"))));
                resource
            }
        };
        if let Err(errors) = bundle.add_resource(resource) {
            problems.extend(errors.iter().map(|e| problem(format!("{e:?}"))));
        }
    }
    Layer { namespace: source.namespace.clone(), locale, bundle }
}
