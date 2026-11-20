//! Ctrl+T: the Cart view, where the user picks the Target Cart and types the Claim. Each visit is
//! one undo step, and Esc restores the Target Cart as it was when the view opened.

use super::files::{Waiting, cannot_use};
use super::focus::Focus;
use super::messages::Report;
use super::panes::cart_view::{CartField, CartView};
use super::panes::file_list::Purpose;
use super::paths::relative_path;
use super::{Action, App, Direction};
use crate::cart::Cart;
use crate::export;
use crate::project::{Claim, TargetCart};
use std::path::Path;

const PICK_A_CART_FIRST: &str = "PICK A TARGET CART FIRST";

impl App {
    /// Whether the Cart view is showing in the right-hand slot and has focus.
    pub(super) fn cart_view_has_focus(&self) -> bool {
        self.cart_view.is_some() && self.focus == Focus::RightSlot && !self.log_open
    }

    pub(super) fn toggle_cart(&mut self) {
        if self.cart_view.is_some() {
            self.close_cart_view();
        } else {
            self.open_cart_view();
        }
    }

    /// Opens the Cart view with focus, and asks for the Target Cart's bytes to draw its maps.
    pub(super) fn open_cart_view(&mut self) {
        self.log_open = false;
        self.cart_visits += 1;
        let view = CartView::new(self.project.target_cart(), self.played_pane);
        self.cart_view = Some(view);
        self.set_focus(Focus::RightSlot);
        if let Some(target) = self.project.target_cart() {
            let path = self.target_cart_path(&target.path);
            self.request_bytes(path.clone());
            self.waiting = Some(Waiting::CartViewReading(path));
        }
    }

    /// Closes the Cart view, keeping the Claim, and gives focus back to the pane it came from.
    pub(super) fn close_cart_view(&mut self) {
        let Some(view) = self.cart_view.take() else {
            return;
        };
        if self.focus == Focus::RightSlot && !self.log_open {
            self.set_focus(view.focus_before());
        }
    }

    /// What an action does in the Cart view, if it does anything there.
    pub(super) fn use_cart_view(&mut self, action: Action) -> bool {
        let Some(view) = self.cart_view.as_mut() else {
            return false;
        };
        match action {
            Action::Move(direction @ (Direction::Up | Direction::Down)) => {
                view.move_cursor(direction);
            }
            Action::HexDigit(digit) => self.edit_claim(|view| view.type_digit(digit)),
            Action::TypeChar('-') => self.edit_claim(CartView::type_dash),
            Action::Move(_) | Action::Clear | Action::TypeChar(_) => {}
            Action::Backspace => self.edit_claim(CartView::backspace),
            Action::SuggestClaim => self.suggest_claim(),
            Action::Confirm => match view.field() {
                CartField::Cart => self.browse(Purpose::TargetCart),
                CartField::Sfx => view.move_cursor(Direction::Down),
                CartField::Patterns => self.close_cart_view(),
            },
            Action::Cancel => {
                let opened_with = view.opened_with().cloned();
                self.project.set_target_cart(opened_with);
                self.close_cart_view();
            }
            Action::ToggleCart => self.close_cart_view(),
            _ => return false,
        }
        true
    }

    /// Edits the text of a Claim field, putting in force whatever it now holds that is whole.
    fn edit_claim(&mut self, edit: impl FnOnce(&mut CartView)) {
        let Some(view) = self.cart_view.as_mut() else {
            return;
        };
        if view.field() == CartField::Cart {
            return;
        }
        let Some(mut target) = self.project.target_cart().cloned() else {
            self.remark = Some(PICK_A_CART_FIRST);
            return;
        };
        edit(view);
        target.claim = view.typed_claim(&target.claim);
        self.project.set_target_cart(Some(target));
    }

    /// `=`: claims what the Target Cart's free slots suggest.
    fn suggest_claim(&mut self) {
        let Some(target) = self.project.target_cart() else {
            self.remark = Some(PICK_A_CART_FIRST);
            return;
        };
        self.apply_suggestion(target.clone());
    }

    /// Gives the Target Cart the Claim its free slots suggest, once the view knows them.
    fn apply_suggestion(&mut self, mut target: TargetCart) {
        let Some(view) = self.cart_view.as_mut() else {
            return;
        };
        let needs = export::needs(&self.project);
        let hash = target.claimed_bytes_hash.as_deref();
        if let Some(claim) = view.suggested_claim(hash, needs) {
            view.show_claim(&claim);
            target.claim = claim;
        }
        self.project.set_target_cart(Some(target));
    }

    /// Shows the Claim in force, after undo or redo has changed it.
    pub(super) fn show_claim_in_cart_view(&mut self) {
        let claim = self
            .project
            .target_cart()
            .map(|target| target.claim.clone())
            .unwrap_or_default();
        if let Some(view) = self.cart_view.as_mut() {
            view.show_claim(&claim);
        }
    }

    pub(super) fn show_cart_in_view(&mut self, contents: &[u8]) {
        if let Some(view) = self.cart_view.as_mut() {
            view.show_contents(Cart::read(contents).map_err(|error| error.to_string()));
        }
    }

    pub(super) fn show_unreadable_cart_in_view(&mut self, reason: String) {
        if let Some(view) = self.cart_view.as_mut() {
            view.show_contents(Err(reason));
        }
    }

    /// A path as the Target Cart stores it: from the folder of the Project file.
    pub(super) fn path_from_project(&self, path: &Path) -> String {
        relative_path(self.target_cart_folder(), path)
            .unwrap_or_else(|| path.to_path_buf())
            .to_string_lossy()
            .into_owned()
    }

    /// Makes a Cart the user picked the Target Cart, with the Claim its free slots suggest.
    pub(super) fn finish_picking_cart(&mut self, path: &Path, contents: &[u8]) {
        self.file_list = None;
        let cart = match Cart::read(contents) {
            Ok(cart) => cart,
            Err(error) => {
                self.report(Report::failure(cannot_use(path, &error.to_string())));
                return;
            }
        };
        let path = self.path_from_project(path);
        // The same Cart picked again still holds what the last Export wrote.
        let claimed_bytes_hash = self
            .project
            .target_cart()
            .filter(|target| target.path == path)
            .and_then(|target| target.claimed_bytes_hash.clone());
        let target = TargetCart {
            path,
            claim: Claim::default(),
            claimed_bytes_hash,
        };
        if let Some(view) = self.cart_view.as_mut() {
            view.show_contents(Ok(cart));
            view.move_cursor(Direction::Down);
        }
        self.apply_suggestion(target);
    }
}
