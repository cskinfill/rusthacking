// @generated automatically by Diesel CLI.

diesel::table! {
    services (id) {
        id -> Integer,
        name -> Nullable<Text>,
        description -> Nullable<Text>,
        versions -> Nullable<Integer>,
    }
}
