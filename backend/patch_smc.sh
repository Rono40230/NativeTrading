sed -i '30s/pub sauvegarde: bool,/pub sauvegarde: bool,\n    pub importances: Vec<crate::rockets_trainer::ImportanceFeature>,/' crates/ml/src/smc_trainer.rs
