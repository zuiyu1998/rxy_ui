mod derived;
mod resource;

use crate::{arena::Owner, source::AnySubscriber, Observer};
pub use derived::*;
use futures::Future;
use pin_project_lite::pin_project;
pub use resource::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AsyncState<T> {
    #[default]
    Loading,
    Complete(T),
    Reloading(T),
}

impl<T> AsyncState<T> {
    pub fn current_value(&self) -> Option<&T> {
        match &self {
            AsyncState::Loading => None,
            AsyncState::Complete(val) | AsyncState::Reloading(val) => Some(val),
        }
    }

    pub fn loading(&self) -> bool {
        matches!(&self, AsyncState::Loading | AsyncState::Reloading(_))
    }
}

pin_project! {
    pub struct ScopedFuture<Fut> {
        owner: Option<Owner>,
        observer: Option<AnySubscriber>,
        #[pin]
        fut: Fut,
    }
}

impl<Fut> ScopedFuture<Fut> {
    pub fn new(fut: Fut) -> Self {
        let owner = Owner::current();
        let observer = Observer::get();
        Self {
            owner,
            observer,
            fut,
        }
    }
}

impl<Fut: Future> Future for ScopedFuture<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match (this.owner, this.observer) {
            (None, None) => this.fut.poll(cx),
            (None, Some(obs)) => obs.with_observer(|| this.fut.poll(cx)),
            (Some(owner), None) => owner.with_cleanup(|| this.fut.poll(cx)),
            (Some(owner), Some(observer)) => owner
                .with_cleanup(|| observer.with_observer(|| this.fut.poll(cx))),
        }
    }
}
