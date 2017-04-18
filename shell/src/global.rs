use search;
use job;

pub struct State {
	pub search_cache: search::SearchCache,
	pub job_set: job::JobSet,
}

impl State {
	pub fn new() -> State {
		let search_cache = search::SearchCache::new();
		let job_set = job::JobSet::new();
		State { search_cache: search_cache, job_set: job_set }
	}
}
