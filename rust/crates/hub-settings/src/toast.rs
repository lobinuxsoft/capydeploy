/// Default toast display duration in milliseconds.
const DEFAULT_DURATION_MS: u64 = 4000;

/// Error toast display duration in milliseconds (longer for visibility).
const ERROR_DURATION_MS: u64 = 6000;

/// The visual category of a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

/// A toast notification for the Hub UI.
#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub toast_type: ToastType,
    pub title: String,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// In-memory toast notification queue with monotonic ID assignment.
///
/// Timer-based auto-dismissal is a UI concern (iced `Subscription`).
/// This struct just holds the queue and exposes add/remove.
#[derive(Debug, Clone)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
    next_id: u64,
}

impl ToastQueue {
    /// Create an empty toast queue.
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            next_id: 0,
        }
    }

    /// Push a toast with explicit type, title, message, and duration.
    /// Returns the assigned toast ID.
    pub fn push(
        &mut self,
        toast_type: ToastType,
        title: impl Into<String>,
        message: Option<String>,
        duration_ms: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.toasts.push(Toast {
            id,
            toast_type,
            title: title.into(),
            message,
            duration_ms,
        });
        id
    }

    /// Push a success toast with default duration (4s).
    pub fn success(&mut self, title: impl Into<String>) -> u64 {
        self.push(ToastType::Success, title, None, DEFAULT_DURATION_MS)
    }

    /// Push an error toast with extended duration (6s).
    pub fn error(&mut self, title: impl Into<String>) -> u64 {
        self.push(ToastType::Error, title, None, ERROR_DURATION_MS)
    }

    /// Push a warning toast with default duration (4s).
    pub fn warning(&mut self, title: impl Into<String>) -> u64 {
        self.push(ToastType::Warning, title, None, DEFAULT_DURATION_MS)
    }

    /// Push an info toast with default duration (4s).
    pub fn info(&mut self, title: impl Into<String>) -> u64 {
        self.push(ToastType::Info, title, None, DEFAULT_DURATION_MS)
    }

    /// Push a success toast with body text and default duration (4s).
    pub fn success_with(&mut self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(
            ToastType::Success,
            title,
            Some(message.into()),
            DEFAULT_DURATION_MS,
        )
    }

    /// Push an error toast with body text and extended duration (6s).
    pub fn error_with(&mut self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(
            ToastType::Error,
            title,
            Some(message.into()),
            ERROR_DURATION_MS,
        )
    }

    /// Push a warning toast with body text and default duration (4s).
    pub fn warning_with(&mut self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(
            ToastType::Warning,
            title,
            Some(message.into()),
            DEFAULT_DURATION_MS,
        )
    }

    /// Push an info toast with body text and default duration (4s).
    pub fn info_with(&mut self, title: impl Into<String>, message: impl Into<String>) -> u64 {
        self.push(
            ToastType::Info,
            title,
            Some(message.into()),
            DEFAULT_DURATION_MS,
        )
    }

    /// Remove a toast by ID. Returns `true` if found and removed.
    pub fn remove(&mut self, id: u64) -> bool {
        let len_before = self.toasts.len();
        self.toasts.retain(|t| t.id != id);
        self.toasts.len() != len_before
    }

    /// Look up a toast by ID.
    pub fn get(&self, id: u64) -> Option<&Toast> {
        self.toasts.iter().find(|t| t.id == id)
    }

    /// Iterate over toasts in insertion order (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter()
    }

    /// Number of toasts currently in the queue.
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Remove all toasts.
    pub fn clear(&mut self) {
        self.toasts.clear();
    }
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_retrieve() {
        let mut q = ToastQueue::new();
        let id = q.push(ToastType::Info, "Test", None, 3000);

        let toast = q.get(id).unwrap();
        assert_eq!(toast.title, "Test");
        assert_eq!(toast.toast_type, ToastType::Info);
        assert_eq!(toast.duration_ms, 3000);
        assert!(toast.message.is_none());
    }

    #[test]
    fn push_multiple_correct_order_and_ids() {
        let mut q = ToastQueue::new();
        let id0 = q.success("first");
        let id1 = q.error("second");
        let id2 = q.info("third");

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        let titles: Vec<&str> = q.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["first", "second", "third"]);
    }

    #[test]
    fn remove_toast() {
        let mut q = ToastQueue::new();
        let id0 = q.success("keep");
        let id1 = q.error("remove me");
        let _id2 = q.info("also keep");

        assert!(q.remove(id1));
        assert!(q.get(id1).is_none());
        assert_eq!(q.len(), 2);

        // Remaining toasts intact
        assert!(q.get(id0).is_some());
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut q = ToastQueue::new();
        q.success("a");

        assert!(!q.remove(999));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn convenience_methods_set_correct_types() {
        let mut q = ToastQueue::new();
        let s = q.success("s");
        let e = q.error("e");
        let w = q.warning("w");
        let i = q.info("i");

        assert_eq!(q.get(s).unwrap().toast_type, ToastType::Success);
        assert_eq!(q.get(e).unwrap().toast_type, ToastType::Error);
        assert_eq!(q.get(w).unwrap().toast_type, ToastType::Warning);
        assert_eq!(q.get(i).unwrap().toast_type, ToastType::Info);
    }

    #[test]
    fn error_duration_differs_from_default() {
        let mut q = ToastQueue::new();
        let s = q.success("ok");
        let e = q.error("fail");
        let w = q.warning("hmm");
        let i = q.info("fyi");

        assert_eq!(q.get(s).unwrap().duration_ms, 4000);
        assert_eq!(q.get(e).unwrap().duration_ms, 6000);
        assert_eq!(q.get(w).unwrap().duration_ms, 4000);
        assert_eq!(q.get(i).unwrap().duration_ms, 4000);
    }

    #[test]
    fn with_message_variants() {
        let mut q = ToastQueue::new();
        let id = q.success_with("Title", "Body text");

        let toast = q.get(id).unwrap();
        assert_eq!(toast.title, "Title");
        assert_eq!(toast.message.as_deref(), Some("Body text"));
        assert_eq!(toast.toast_type, ToastType::Success);
    }

    #[test]
    fn error_with_has_extended_duration() {
        let mut q = ToastQueue::new();
        let id = q.error_with("Oops", "Details here");

        assert_eq!(q.get(id).unwrap().duration_ms, 6000);
    }

    #[test]
    fn clear_removes_all() {
        let mut q = ToastQueue::new();
        q.success("a");
        q.error("b");
        q.info("c");

        q.clear();

        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn len_and_is_empty() {
        let mut q = ToastQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);

        q.success("a");
        assert!(!q.is_empty());
        assert_eq!(q.len(), 1);
    }
}
