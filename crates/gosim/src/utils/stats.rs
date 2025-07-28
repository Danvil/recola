#[macro_export]
macro_rules! stat_component {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $crate::decimal_component! {
            $(#[$meta])*
            $name
        }

        paste::paste! {

            impl $name {
                pub fn setup(world: &World) {
                    world.component::<Self>();
                    world.component::<[<$name Base>]>();
                    world.component::<[<$name Mods>]>();
                    world
                        .system::<(&[<$name Base>], &[<$name Mods>], &mut Self)>()
                        .each(|(base, modf, eff)| {
                            **eff = base.value() * modf.factor();
                        });
                }
            }

            /// Base for stat [`$name`]
            #[derive(Component, Debug, Clone, Copy)]
            pub struct [<$name Base>] {
                value: f64,
            }

            impl [<$name Base>] {
                pub fn new(value: f64) -> Self {
                    Self {
                        value,
                    }
                }

                pub fn value(&self) -> f64 {
                    self.value
                }
            }

            /// Modifiers for stat [`$name`]
            #[derive(Component, Debug, Clone)]
            pub struct [<$name Mods>] {
                add: std::collections::HashMap<String, f64>,
                more: std::collections::HashMap<String, f64>,
                factor: f64,
            }

            impl Default for [<$name Mods>] {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl [<$name Mods>] {
                pub fn new() -> Self {
                    Self {
                        add: std::collections::HashMap::new(),
                        more: std::collections::HashMap::new(),
                        factor: 1.,
                    }
                }

                pub fn set_add_mod(&mut self, kind: &str, add: f64) {
                    if add == 0. {
                        self.add.remove(kind);
                    } else {
                        self.add.insert(kind.to_string(), add);
                    }
                    self.update_factor();
                }

                pub fn set_more_mod(&mut self, kind: &str, more: f64) {
                    if more == 0. {
                        self.more.remove(kind);
                    } else {
                        self.more.insert(kind.to_string(), more);
                    }
                    self.update_factor();
                }

                pub fn factor(&self) -> f64 {
                    self.factor
                }

                pub fn set_mod(&mut self, kind: &str, modifier: gems::Modifier) {
                    match modifier.kind() {
                        gems::ModifierKind::Additive => self.set_add_mod(kind, modifier.value()),
                        gems::ModifierKind::More => self.set_more_mod(kind, modifier.value()),
                    }
                    self.update_factor();
                }

                fn update_factor(&mut self) {
                    let add: f64 = self.add.iter().map(|(_, v)| v).sum();
                    let more: f64 = self
                        .more
                        .iter()
                        .map(|(_, v)| (1. + v).max(0.))
                        .product();
                    self.factor = (1. + add).max(0.) * more;
                }
            }
        }
    };
}
