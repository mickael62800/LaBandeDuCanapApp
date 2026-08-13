//! Catalogue pur des objets Coussin Piégé.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopItem {
    pub key: &'static str,
    pub name: &'static str,
    pub price: i64,
    pub description: &'static str,
}

/// Les cles restent celles d'origine : elles sont ecrites dans les
/// inventaires deja constitues. Renommer l'affichage est gratuit, renommer
/// une cle viderait le sac de tout le monde.
pub const ITEMS: &[ShopItem] = &[
    ShopItem {
        key: "rage",
        name: "Coussin Plombe",
        price: 100,
        description: "+50 d'impact pour une bagarre",
    },
    ShopItem {
        key: "mindgame",
        name: "Oeil sous le Plaid",
        price: 150,
        description: "Revele le jet d'en face",
    },
    ShopItem {
        key: "explosion",
        name: "Renversement de Chips",
        price: 200,
        description: "Annule le gain de la bagarre",
    },
    ShopItem {
        key: "double_coup",
        name: "Double Coussin",
        price: 250,
        description: "Garde le meilleur de deux jets",
    },
    ShopItem {
        key: "surprise",
        name: "Bataille d'Oreillers",
        price: 300,
        description: "Defi immediat, sans prevenir",
    },
    ShopItem {
        key: "coup_traitre",
        name: "Punaise dans le Coussin",
        price: 350,
        description: "Ignore le moelleux adverse",
    },
    ShopItem {
        key: "inversion",
        name: "Retourne le Canape",
        price: 500,
        description: "Echange les soldes",
    },
];

pub fn item(key: &str) -> Option<ShopItem> {
    ITEMS.iter().copied().find(|item| item.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_valid_item() {
        let rage = item("rage");
        assert!(rage.is_some());
        let item = rage.unwrap();
        assert_eq!(item.name, "Coussin Plombe");
        assert_eq!(item.price, 100);
    }

    #[test]
    fn test_find_unknown_item() {
        assert!(item("unknown_item_key").is_none());
    }

    #[test]
    fn test_all_items_have_unique_keys_and_positive_prices() {
        let mut keys = std::collections::HashSet::new();
        for item in ITEMS {
            assert!(item.price > 0, "Item {} must have positive price", item.key);
            assert!(!item.key.is_empty(), "Item key cannot be empty");
            assert!(keys.insert(item.key), "Duplicate key: {}", item.key);
        }
    }
}
