pub(crate) use futures::channel::mpsc::Receiver;
use futures::{channel::mpsc::channel, SinkExt};
use notify::{Config, Event, RecursiveMode, Result};
pub(crate) use notify::{INotifyWatcher, PollWatcher, Watcher};

use std::path::Path;
pub use std::time::Duration;

pub fn pseudo_fs_watcher<P: AsRef<Path>>(
    path: P,
    poll_interval: Duration,
) -> Result<(PollWatcher, INotifyWatcher, Receiver<Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let config = Config::default()
        .with_compare_contents(true) // crucial part for pseudo filesystems
        .with_poll_interval(poll_interval);

    // PollWatcher is used to observe the devices as they come/go
    let mut poll_tx = tx.clone();
    let mut poll_watcher = PollWatcher::new(
        move |res: notify::Result<Event>| {
            futures::executor::block_on(async {
                poll_tx.send(res).await.unwrap();
            });
        },
        config,
    )?;

    // INotifyWatcher watches the contents of the files
    let inotify_watcher = INotifyWatcher::new(
        move |res: notify::Result<Event>| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            });
        },
        config,
    )?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    poll_watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    Ok((poll_watcher, inotify_watcher, rx))
}
