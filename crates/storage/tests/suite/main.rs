//! The suite every backend of the storage interface runs.
//!
//! Each case in cases.rs is written once, and runs once per backend: git,
//! against a real repository in a temporary directory, and memory. A case
//! that one backend fails is a promise that backend does not keep. git.rs
//! holds what only git does, such as ignore rules and conflicts.

mod cases;
mod fixture;
mod git;

/// One test per case per backend, so a failure names both.
macro_rules! every_backend {
    ($($case:ident),+ $(,)?) => {
        mod on_git {
            $( #[test] fn $case() { crate::cases::$case(&mut crate::fixture::GitRepo::new()) } )+
        }
        mod on_memory {
            $( #[test] fn $case() { crate::cases::$case(&mut storage::memory::Memory::new()) } )+
        }
    };
}

every_backend!(
    a_repository_with_no_revision_has_no_head_and_no_history,
    before_the_first_revision_every_file_is_added_or_untracked,
    declarations_are_every_file_inside_a_pile_directory,
    declarations_at_a_revision_are_what_it_recorded,
    the_files_of_a_subtree_leave_out_what_it_excepts,
    a_file_is_listed_with_its_kind,
    a_read_answers_each_path_in_order_and_none_where_no_file_is,
    the_changes_between_two_revisions_name_each_path_once,
    from_the_empty_repository_every_file_is_added,
    the_working_copy_is_compared_with_any_base,
    a_move_in_the_working_copy_is_a_move,
    a_history_is_newest_first_and_counts_what_it_leaves_out,
    a_history_leaves_out_what_the_subtree_excepts,
    a_move_in_a_history_is_both_of_its_paths,
    a_revision_says_who_made_it_when_and_why,
    a_revision_resolves_by_its_own_id,
    the_bases_start_at_the_head_and_go_back_along_its_branch,
    the_merge_base_of_two_revisions_on_one_line_is_the_older,
);
