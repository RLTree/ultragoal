//! Transport for a chunked check. A check whose selected content exceeds the
//! evaluation-frame budget streams that content in bounded CHUNK frames to cores
//! of its own; Bend computes each chunk's partials and folds them in the main
//! evaluation. This module only groups rows into frames and forwards Bend's rows.
use super::*;
use std::collections::BTreeSet;
use std::sync::mpsc;

/// Selected content above this travels in chunks instead of the evaluation frame.
pub const THRESHOLD: u64 = 2 * 1024 * 1024;
/// Encoded F-row bytes per chunk frame.
pub const CHUNK_BYTES: usize = 1024 * 1024;
/// A core request's frames are evaluated in parallel, so a request's bytes and rows
/// bound the core's working set; the core holds about 7 KB per row.
const REQUEST_BYTES: usize = 8 * 1024 * 1024;
const REQUEST_FRAMES: usize = 8;
const REQUEST_ROWS: usize = 16 * 1024;
pub const FRAME_ROWS: usize = REQUEST_ROWS / REQUEST_FRAMES;

pub fn selected_bytes(listing: &fs_adapter::Listing, wanted: &BTreeSet<String>) -> u64 {
    listing
        .members
        .iter()
        .filter(|m| std::str::from_utf8(&m.path).is_ok_and(|p| wanted.contains(p)))
        .map(|m| m.identity.size)
        .sum()
}

/// Whether a file's row cannot fit one chunk frame; it is then an unavailable input.
pub fn oversized(header: usize, path: &str, content: &str) -> bool {
    header + 96 + enc_len(path) + enc_len(content) > MAX
}

struct Request {
    frames: Vec<String>,
    first: String,
    last: String,
}

/// Groups F rows, in order, into chunk frames of at most CHUNK_BYTES and FRAME_ROWS,
/// and frames into requests of at most REQUEST_FRAMES, REQUEST_BYTES and REQUEST_ROWS.
struct Batcher {
    header: String,
    rows: String,
    bytes: usize,
    count: usize,
    first: String,
    last: String,
    group: Request,
    group_bytes: usize,
    group_rows: usize,
    frames: usize,
}

impl Batcher {
    fn new(header: String) -> Self {
        let group = Request { frames: Vec::new(), first: String::new(), last: String::new() };
        Self { header, rows: String::new(), bytes: 0, count: 0, first: String::new(), last: String::new(), group, group_bytes: 0, group_rows: 0, frames: 0 }
    }

    fn push(&mut self, path: &str, row: String) -> Option<Request> {
        let ready = if self.count > 0 && (self.bytes + row.len() > CHUNK_BYTES || self.count == FRAME_ROWS) {
            self.close()
        } else {
            None
        };
        if self.count == 0 {
            self.first = path.to_string();
        }
        self.last = path.to_string();
        self.bytes += row.len();
        self.count += 1;
        self.rows += &row;
        ready
    }

    /// Closes the current frame; returns the group it could not join.
    fn close(&mut self) -> Option<Request> {
        let frame = format!("{}{}{}", self.header, row(&["CI", &self.frames.to_string()]), std::mem::take(&mut self.rows));
        self.frames += 1;
        let full = !self.group.frames.is_empty()
            && (self.group.frames.len() == REQUEST_FRAMES
                || self.group_bytes + self.bytes > REQUEST_BYTES
                || self.group_rows + self.count > REQUEST_ROWS);
        let ready = full.then(|| self.take());
        if self.group.frames.is_empty() {
            self.group.first = std::mem::take(&mut self.first);
        }
        self.group.last = std::mem::take(&mut self.last);
        self.group.frames.push(frame);
        self.group_bytes += self.bytes;
        self.group_rows += self.count;
        (self.bytes, self.count) = (0, 0);
        ready
    }

    fn take(&mut self) -> Request {
        (self.group_bytes, self.group_rows) = (0, 0);
        std::mem::replace(&mut self.group, Request { frames: Vec::new(), first: String::new(), last: String::new() })
    }

    fn finish(&mut self) -> Vec<Request> {
        let mut out = Vec::new();
        if self.count > 0 || self.frames == 0 {
            out.extend(self.close());
        }
        if !self.group.frames.is_empty() {
            out.push(self.take());
        }
        out
    }
}

pub struct Chunked {
    pub rows: String,
    pub frames: usize,
    pub requests: usize,
    pub cores: usize,
    pub ms: u128,
}

type Answer = Result<(String, u128), String>;

/// Evaluates requests in order on a dedicated lane; each answer holds Bend's CH, CF,
/// PC and PW rows verbatim and the request's wall time.
fn lane(requests: mpsc::Receiver<Request>, answers: mpsc::Sender<Answer>) -> Result<usize, String> {
    let mut lane = core_session::Lane::new("content chunk")?;
    let mut used = 0usize;
    for request in requests {
        let started = Instant::now();
        let count = request.frames.len();
        let answer = lane
            .request(request.frames, REPORT_MAX - used)
            .map_err(|e| format!("{e}; paths {} .. {}", request.first, request.last))
            .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "core UTF-8".to_string()))
            .and_then(|output| {
                if let Some(line) = output.lines().find(|l| !["CH\t", "CF\t", "PC\t", "PW\t"].iter().any(|t| l.starts_with(t))) {
                    return Err(format!("Bend rejected content chunk: {}", line.chars().take(200).collect::<String>()));
                }
                if output.lines().filter(|l| l.starts_with("CH\t")).count() != count {
                    return Err("incomplete content chunk response".into());
                }
                Ok(output)
            });
        used += answer.as_ref().map_or(0, String::len);
        let failed = answer.is_err();
        if answers.send(answer.map(|o| (o, started.elapsed().as_millis()))).is_err() || failed {
            return Ok(0);
        }
    }
    lane.finish()
}

/// Sends F rows, as they are pushed, in CHUNK requests on a dedicated lane. The lane
/// evaluates one request while the next is captured, so at most two are alive.
pub struct Stream {
    batcher: Batcher,
    requests: Option<mpsc::SyncSender<Request>>,
    answers: mpsc::Receiver<Answer>,
    worker: Option<std::thread::JoinHandle<Result<usize, String>>>,
    output: String,
    sent: usize,
    received: usize,
    ms: u128,
}

impl Stream {
    pub fn new(header: String) -> Self {
        let (requests, queue) = mpsc::sync_channel(0);
        let (done, answers) = mpsc::channel();
        let worker = std::thread::spawn(move || lane(queue, done));
        Self { batcher: Batcher::new(header), requests: Some(requests), answers, worker: Some(worker), output: String::new(), sent: 0, received: 0, ms: 0 }
    }

    pub fn push(&mut self, path: &str, row: String) -> Result<(), String> {
        match self.batcher.push(path, row) {
            Some(request) => self.dispatch(request),
            None => Ok(()),
        }
    }

    fn take(&mut self, answer: Answer) -> Result<(), String> {
        let (output, ms) = answer?;
        self.output += &output;
        self.ms += ms;
        self.received += 1;
        Ok(())
    }

    fn dispatch(&mut self, request: Request) -> Result<(), String> {
        let sent = self.requests.as_ref().is_some_and(|q| q.send(request).is_ok());
        if !sent {
            return Err(self.stop().err().unwrap_or_else(|| "content chunk lane stopped".into()));
        }
        self.sent += 1;
        while let Ok(answer) = self.answers.try_recv() {
            self.take(answer)?;
        }
        Ok(())
    }

    /// Closes the lane and collects every answer; returns the cores it used.
    fn stop(&mut self) -> Result<usize, String> {
        self.requests.take();
        while let Ok(answer) = self.answers.recv() {
            self.take(answer)?;
        }
        let worker = self.worker.take().ok_or("content chunk lane absent")?;
        worker.join().map_err(|_| "content chunk lane panicked")?
    }

    pub fn finish(mut self) -> Result<Chunked, String> {
        for request in self.batcher.finish() {
            self.dispatch(request)?;
        }
        let cores = self.stop()?;
        if self.received != self.sent {
            return Err("incomplete content chunk response".into());
        }
        Ok(Chunked { rows: std::mem::take(&mut self.output), frames: self.batcher.frames, requests: self.sent, cores, ms: self.ms })
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_stay_within_frame_byte_and_row_bounds_with_contiguous_ids() {
        let mut batcher = Batcher::new("CHUNK\nUG\t1\n".into());
        let mut requests = Vec::new();
        let sizes = [(40usize, 40_000usize), (300_000, 30), (20, 3_000), (900_000, 12), (10, 20_000)];
        let mut n = 0;
        for (size, count) in sizes {
            for _ in 0..count {
                let path = format!("f{n:07}");
                n += 1;
                requests.extend(batcher.push(&path, row(&["F", &path, "unread", &"x".repeat(size)])));
            }
        }
        requests.extend(batcher.finish());
        let mut next = 0;
        let mut seen = 0;
        for request in &requests {
            assert!(!request.frames.is_empty() && request.frames.len() <= REQUEST_FRAMES);
            let mut bytes = 0;
            let mut rows = 0;
            for frame in &request.frames {
                let body = frame.strip_prefix("CHUNK\nUG\t1\n").unwrap();
                let (ci, rest) = body.split_once('\n').unwrap();
                assert_eq!(ci, format!("CI\t{next}"));
                next += 1;
                assert!(rest.len() <= CHUNK_BYTES || rest.lines().count() == 1);
                bytes += rest.len();
                rows += rest.lines().count();
            }
            assert!(bytes <= REQUEST_BYTES, "{bytes}");
            assert!(rows <= REQUEST_ROWS, "{rows}");
            let first = request.frames[0].lines().nth(3).unwrap().split('\t').nth(1).unwrap();
            let last = request.frames.last().unwrap().lines().last().unwrap().split('\t').nth(1).unwrap();
            assert_eq!((first, last), (request.first.as_str(), request.last.as_str()));
            seen += rows;
        }
        assert_eq!((next, seen), (batcher.frames, n));
        assert!(requests.len() >= 5, "{}", requests.len());
    }
}
