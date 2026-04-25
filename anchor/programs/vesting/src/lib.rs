use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[cfg(test)]
mod tests;

declare_id!("EHPUBQcoqciVo4iWdJ9ppU1xvMt7pg3V4ecVdkHCYb1v");

#[program]
pub mod vesting {
    use super::*;

}