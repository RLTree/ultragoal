mod contracts;
mod emitter;
mod filesystem;
mod packet_parser;
mod projection;

pub(crate) use contracts::RedCatalogProjectionRequest;
pub(crate) use projection::check;
#[cfg(test)]
pub(crate) use projection::render;

#[cfg(test)]
mod tests;
