//! 播放控制命令通道（M1 §17–18）：Control Thread → Command Queue → Engine。
//!
//! SPSC rtrb 队列；控制面 `post`，引擎在渲染循环中 `drain`。命令无返回通道：
//! 非法命令被安全忽略并计数（M1 §16），错误经计数器/状态可观测。

use rtrb::Producer;

use crate::error::AudioError;
use crate::types::AudioCommand;

/// 命令队列控制端。
pub struct AudioCommandQueue {
    inner: Producer<AudioCommand>,
    capacity: usize,
}

impl AudioCommandQueue {
    /// crate 内构造（Engine::new 与队列是配对创建的）。
    pub(crate) fn new(inner: Producer<AudioCommand>, capacity: usize) -> Self {
        Self { inner, capacity }
    }

    /// 投递命令；队列满返回 `CommandQueueError::Full`（控制面自行重试）。
    pub fn post(&mut self, command: AudioCommand) -> Result<(), CommandQueueError> {
        self.inner
            .push(command)
            .map_err(|_| CommandQueueError::Full)
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandQueueError {
    Full,
}

impl AudioError {
    /// 便捷：队列错误转 `AudioError`。
    #[must_use]
    pub const fn from_queue_error(e: CommandQueueError) -> Self {
        match e {
            CommandQueueError::Full => AudioError::InvalidCommand("command queue full"),
        }
    }
}
