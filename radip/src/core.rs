//! Provides the [`Core`] order type, as found in some variants.

use serde::{Deserialize, Serialize};

use crate::{base, Order};

/// A core order. Succeeds if untapped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Core;

#[typetag::serde(name = "core")]
impl Order for Core {
    fn deps(
            &self,
            map: &crate::Map,
            state: &crate::MapState,
            orders: &crate::Orders,
            this_prov: &str,
        ) -> std::collections::HashSet<String> {
        base::deps_for_tap(map, state, orders, this_prov)
    }

    fn adjudicate(
            &self,
            map: &crate::Map,
            state: &crate::MapState,
            orders: &crate::Orders,
            this_prov: &str,
            order_status: &std::collections::HashMap<String, bool>,
        ) -> Option<bool> {
        base::is_untapped(map, state, orders, order_status, this_prov, "")
    }

    fn as_owned(&self) -> Box<dyn Order> {
        Box::new(self.clone())
    }
}