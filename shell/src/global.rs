use search;
use job;

pub struct State {
	pub search_cache: search::SearchCache,
	pub job_set: job::JobSet,
}
