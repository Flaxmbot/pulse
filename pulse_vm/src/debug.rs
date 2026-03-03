//! Debug context for Pulse VM
//! Provides breakpoint management, step modes, and debugging state.

use std::collections::HashSet;

/// Debug execution modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepMode {
    /// Normal execution, only stop at breakpoints
    #[default]
    Continue,
    /// Step one instruction
    StepIn,
    /// Step over function calls (same frame depth)
    StepOver { target_depth: usize },
    /// Step out of current function
    StepOut { target_depth: usize },
}

/// Debug context containing debugging state
#[derive(Debug, Clone)]
pub struct DebugContext {
    /// Breakpoints by instruction pointer
    pub breakpoints_ip: HashSet<usize>,
    /// Breakpoints by source line number
    pub breakpoints_line: HashSet<usize>,
    /// Current step mode
    pub step_mode: StepMode,
    /// Whether the VM is currently paused
    pub paused: bool,
    /// Last IP where we paused (to avoid re-triggering same breakpoint)
    pub last_pause_ip: Option<usize>,
}

impl Default for DebugContext {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugContext {
    pub fn new() -> Self {
        Self {
            breakpoints_ip: HashSet::new(),
            breakpoints_line: HashSet::new(),
            step_mode: StepMode::Continue,
            paused: false,
            last_pause_ip: None,
        }
    }

    /// Set a breakpoint at an instruction pointer
    pub fn set_breakpoint_ip(&mut self, ip: usize) {
        self.breakpoints_ip.insert(ip);
    }

    /// Set a breakpoint at a source line
    pub fn set_breakpoint_line(&mut self, line: usize) {
        self.breakpoints_line.insert(line);
    }

    /// Remove a breakpoint at an instruction pointer
    pub fn remove_breakpoint_ip(&mut self, ip: usize) -> bool {
        self.breakpoints_ip.remove(&ip)
    }

    /// Remove a breakpoint at a source line
    pub fn remove_breakpoint_line(&mut self, line: usize) -> bool {
        self.breakpoints_line.remove(&line)
    }

    /// Clear all breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints_ip.clear();
        self.breakpoints_line.clear();
    }

    /// Check if we should pause at the given IP and line
    pub fn should_pause(&self, ip: usize, line: usize, frame_depth: usize) -> bool {
        // Don't re-trigger at the same IP we just paused at
        if self.last_pause_ip == Some(ip) && self.step_mode == StepMode::Continue {
            return false;
        }

        // Check breakpoints
        if self.breakpoints_ip.contains(&ip) || self.breakpoints_line.contains(&line) {
            return true;
        }

        // Check step mode
        match self.step_mode {
            StepMode::Continue => false,
            StepMode::StepIn => true,
            StepMode::StepOver { target_depth } => frame_depth <= target_depth,
            StepMode::StepOut { target_depth } => frame_depth < target_depth,
        }
    }

    /// Called when continuing execution
    pub fn resume(&mut self) {
        self.paused = false;
        self.step_mode = StepMode::Continue;
    }

    /// Step one instruction
    pub fn step_in(&mut self) {
        self.paused = false;
        self.step_mode = StepMode::StepIn;
    }

    /// Step over (stay at same or higher frame depth)
    pub fn step_over(&mut self, current_depth: usize) {
        self.paused = false;
        self.step_mode = StepMode::StepOver {
            target_depth: current_depth,
        };
    }

    /// Step out of current function
    pub fn step_out(&mut self, current_depth: usize) {
        self.paused = false;
        self.step_mode = StepMode::StepOut {
            target_depth: current_depth,
        };
    }

    /// Record that we paused at this IP
    pub fn mark_paused(&mut self, ip: usize) {
        self.paused = true;
        self.last_pause_ip = Some(ip);
    }

    /// List all breakpoints
    pub fn list_breakpoints(&self) -> Vec<String> {
        let mut result = Vec::new();
        for &ip in &self.breakpoints_ip {
            result.push(format!("IP: {}", ip));
        }
        for &line in &self.breakpoints_line {
            result.push(format!("Line: {}", line));
        }
        result
    }
}
