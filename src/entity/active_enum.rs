use crate::{ColIdx, ColumnDef, DbErr, Iterable, QueryResult, TryFromU64, TryGetError, TryGetable};
use sea_query::{DynIden, Expr, Nullable, SimpleExpr, Value, ValueType};

/// A Rust representation of enum defined in database.
///
/// # Implementations
///
/// You can implement [ActiveEnum] manually by hand or use the derive macro [DeriveActiveEnum](sea_orm_macros::DeriveActiveEnum).
///
/// # Examples
///
/// Implementing it manually versus using the derive macro [DeriveActiveEnum](sea_orm_macros::DeriveActiveEnum).
///
/// > See [DeriveActiveEnum](sea_orm_macros::DeriveActiveEnum) for the full specification of macro attributes.
///
/// ```rust
/// use sea_orm::entity::prelude::*;
///
/// // Using the derive macro
/// #[derive(Debug, PartialEq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
/// #[sea_orm(
///     rs_type = "String",
///     db_type = "String(Some(1))",
///     enum_name = "category"
/// )]
/// pub enum DeriveCategory {
///     #[sea_orm(string_value = "B")]
///     Big,
///     #[sea_orm(string_value = "S")]
///     Small,
/// }
///
/// // Implementing it manually
/// #[derive(Debug, PartialEq, EnumIter)]
/// pub enum Category {
///     Big,
///     Small,
/// }
///
/// #[derive(Debug, DeriveIden)]
/// pub struct CategoryEnum;
///
/// impl ActiveEnum for Category {
///     // The macro attribute `rs_type` is being pasted here
///     type Value = String;
///
///     type ValueVec = Vec<String>;
///
///     // Will be atomically generated by `DeriveActiveEnum`
///     fn name() -> DynIden {
///         SeaRc::new(CategoryEnum)
///     }
///
///     // Will be atomically generated by `DeriveActiveEnum`
///     fn to_value(&self) -> Self::Value {
///         match self {
///             Self::Big => "B",
///             Self::Small => "S",
///         }
///         .to_owned()
///     }
///
///     // Will be atomically generated by `DeriveActiveEnum`
///     fn try_from_value(v: &Self::Value) -> Result<Self, DbErr> {
///         match v.as_ref() {
///             "B" => Ok(Self::Big),
///             "S" => Ok(Self::Small),
///             _ => Err(DbErr::Type(format!(
///                 "unexpected value for Category enum: {}",
///                 v
///             ))),
///         }
///     }
///
///     fn db_type() -> ColumnDef {
///         // The macro attribute `db_type` is being pasted here
///         ColumnType::String(Some(1)).def()
///     }
/// }
/// ```
///
/// Using [ActiveEnum] on Model.
///
/// ```
/// use sea_orm::entity::prelude::*;
///
/// // Define the `Category` active enum
/// #[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
/// #[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
/// pub enum Category {
///     #[sea_orm(string_value = "B")]
///     Big,
///     #[sea_orm(string_value = "S")]
///     Small,
/// }
///
/// #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #[sea_orm(table_name = "active_enum")]
/// pub struct Model {
///     #[sea_orm(primary_key)]
///     pub id: i32,
///     // Represents a db column using `Category` active enum
///     pub category: Category,
///     pub category_opt: Option<Category>,
/// }
///
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// pub enum Relation {}
///
/// impl ActiveModelBehavior for ActiveModel {}
/// ```
pub trait ActiveEnum: Sized + Iterable {
    /// Define the Rust type that each enum variant corresponds.
    type Value: ActiveEnumValue;

    /// This has no purpose. It will be removed in the next major version.
    type ValueVec;

    /// Get the name of enum
    fn name() -> DynIden;

    /// Convert enum variant into the corresponding value.
    fn to_value(&self) -> Self::Value;

    /// Try to convert the corresponding value into enum variant.
    fn try_from_value(v: &Self::Value) -> Result<Self, DbErr>;

    /// Get the database column definition of this active enum.
    fn db_type() -> ColumnDef;

    /// Convert an owned enum variant into the corresponding value.
    fn into_value(self) -> Self::Value {
        Self::to_value(&self)
    }

    /// Construct a enum expression with casting
    fn as_enum(&self) -> SimpleExpr {
        Expr::val(Self::to_value(self)).as_enum(Self::name())
    }

    /// Get the name of all enum variants
    fn values() -> Vec<Self::Value> {
        Self::iter().map(Self::into_value).collect()
    }
}

/// The Rust Value backing ActiveEnums
pub trait ActiveEnumValue: Into<Value> + ValueType + Nullable + TryGetable {
    /// For getting an array of enum. Postgres only
    fn try_get_vec_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Vec<Self>, TryGetError>;
}

macro_rules! impl_active_enum_value {
    ($type:ident) => {
        impl ActiveEnumValue for $type {
            fn try_get_vec_by<I: ColIdx>(
                _res: &QueryResult,
                _index: I,
            ) -> Result<Vec<Self>, TryGetError> {
                panic!("Not supported by `postgres-array`")
            }
        }
    };
}

macro_rules! impl_active_enum_value_with_pg_array {
    ($type:ident) => {
        impl ActiveEnumValue for $type {
            fn try_get_vec_by<I: ColIdx>(
                _res: &QueryResult,
                _index: I,
            ) -> Result<Vec<Self>, TryGetError> {
                #[cfg(feature = "postgres-array")]
                {
                    <Vec<Self>>::try_get_by(_res, _index)
                }
                #[cfg(not(feature = "postgres-array"))]
                panic!("`postgres-array` is not enabled")
            }
        }
    };
}

impl_active_enum_value!(u8);
impl_active_enum_value!(u16);
impl_active_enum_value!(u32);
impl_active_enum_value!(u64);
impl_active_enum_value_with_pg_array!(String);
impl_active_enum_value_with_pg_array!(i8);
impl_active_enum_value_with_pg_array!(i16);
impl_active_enum_value_with_pg_array!(i32);
impl_active_enum_value_with_pg_array!(i64);

impl<T> TryFromU64 for T
where
    T: ActiveEnum,
{
    fn try_from_u64(_: u64) -> Result<Self, DbErr> {
        Err(DbErr::ConvertFromU64(
            "Fail to construct ActiveEnum from a u64, if your primary key consist of a ActiveEnum field, its auto increment should be set to false."
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate as sea_orm;
    use crate::{error::*, sea_query::SeaRc, *};
    use pretty_assertions::assert_eq;

    #[test]
    fn active_enum_string() {
        #[derive(Debug, PartialEq, Eq, EnumIter)]
        pub enum Category {
            Big,
            Small,
        }

        #[derive(Debug, DeriveIden)]
        #[sea_orm(iden = "category")]
        pub struct CategoryEnum;

        impl ActiveEnum for Category {
            type Value = String;

            type ValueVec = Vec<String>;

            fn name() -> DynIden {
                SeaRc::new(CategoryEnum)
            }

            fn to_value(&self) -> Self::Value {
                match self {
                    Self::Big => "B",
                    Self::Small => "S",
                }
                .to_owned()
            }

            fn try_from_value(v: &Self::Value) -> Result<Self, DbErr> {
                match v.as_ref() {
                    "B" => Ok(Self::Big),
                    "S" => Ok(Self::Small),
                    _ => Err(type_err(format!("unexpected value for Category enum: {v}"))),
                }
            }

            fn db_type() -> ColumnDef {
                ColumnType::String(Some(1)).def()
            }
        }

        #[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
        #[sea_orm(
            rs_type = "String",
            db_type = "String(Some(1))",
            enum_name = "category"
        )]
        pub enum DeriveCategory {
            #[sea_orm(string_value = "B")]
            Big,
            #[sea_orm(string_value = "S")]
            Small,
        }

        assert_eq!(Category::Big.to_value(), "B".to_owned());
        assert_eq!(Category::Small.to_value(), "S".to_owned());
        assert_eq!(DeriveCategory::Big.to_value(), "B".to_owned());
        assert_eq!(DeriveCategory::Small.to_value(), "S".to_owned());

        assert_eq!(
            Category::try_from_value(&"A".to_owned()).err(),
            Some(type_err("unexpected value for Category enum: A"))
        );
        assert_eq!(
            Category::try_from_value(&"B".to_owned()).ok(),
            Some(Category::Big)
        );
        assert_eq!(
            Category::try_from_value(&"S".to_owned()).ok(),
            Some(Category::Small)
        );
        assert_eq!(
            DeriveCategory::try_from_value(&"A".to_owned()).err(),
            Some(type_err("unexpected value for DeriveCategory enum: A"))
        );
        assert_eq!(
            DeriveCategory::try_from_value(&"B".to_owned()).ok(),
            Some(DeriveCategory::Big)
        );
        assert_eq!(
            DeriveCategory::try_from_value(&"S".to_owned()).ok(),
            Some(DeriveCategory::Small)
        );

        assert_eq!(Category::db_type(), ColumnType::String(Some(1)).def());
        assert_eq!(DeriveCategory::db_type(), ColumnType::String(Some(1)).def());

        assert_eq!(
            Category::name().to_string(),
            DeriveCategory::name().to_string()
        );
        assert_eq!(Category::values(), DeriveCategory::values());

        assert_eq!(format!("{}", DeriveCategory::Big), "Big");
        assert_eq!(format!("{}", DeriveCategory::Small), "Small");
    }

    #[test]
    fn active_enum_derive_signed_integers() {
        macro_rules! test_num_value_int {
            ($ident: ident, $rs_type: expr, $db_type: expr, $col_def: ident) => {
                #[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
                #[sea_orm(rs_type = $rs_type, db_type = $db_type)]
                pub enum $ident {
                    #[sea_orm(num_value = -10)]
                    Negative,
                    #[sea_orm(num_value = 1)]
                    Big,
                    #[sea_orm(num_value = 0)]
                    Small,
                }

                test_int!($ident, $rs_type, $db_type, $col_def);
            };
        }

        macro_rules! test_fallback_int {
            ($ident: ident, $fallback_type: ident, $rs_type: expr, $db_type: expr, $col_def: ident) => {
                #[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
                #[sea_orm(rs_type = $rs_type, db_type = $db_type)]
                #[repr(i32)]
                pub enum $ident {
                    Big = 1,
                    Small = 0,
                    Negative = -10,
                }

                test_int!($ident, $rs_type, $db_type, $col_def);
            };
        }

        macro_rules! test_int {
            ($ident: ident, $rs_type: expr, $db_type: expr, $col_def: ident) => {
                assert_eq!($ident::Big.to_value(), 1);
                assert_eq!($ident::Small.to_value(), 0);
                assert_eq!($ident::Negative.to_value(), -10);

                assert_eq!($ident::try_from_value(&1).ok(), Some($ident::Big));
                assert_eq!($ident::try_from_value(&0).ok(), Some($ident::Small));
                assert_eq!($ident::try_from_value(&-10).ok(), Some($ident::Negative));
                assert_eq!(
                    $ident::try_from_value(&2).err(),
                    Some(type_err(format!(
                        "unexpected value for {} enum: 2",
                        stringify!($ident)
                    )))
                );

                assert_eq!($ident::db_type(), ColumnType::$col_def.def());

                assert_eq!(format!("{}", $ident::Big), "Big");
                assert_eq!(format!("{}", $ident::Small), "Small");
                assert_eq!(format!("{}", $ident::Negative), "Negative");
            };
        }

        test_num_value_int!(I8, "i8", "TinyInteger", TinyInteger);
        test_num_value_int!(I16, "i16", "SmallInteger", SmallInteger);
        test_num_value_int!(I32, "i32", "Integer", Integer);
        test_num_value_int!(I64, "i64", "BigInteger", BigInteger);

        test_fallback_int!(I8Fallback, i8, "i8", "TinyInteger", TinyInteger);
        test_fallback_int!(I16Fallback, i16, "i16", "SmallInteger", SmallInteger);
        test_fallback_int!(I32Fallback, i32, "i32", "Integer", Integer);
        test_fallback_int!(I64Fallback, i64, "i64", "BigInteger", BigInteger);
    }

    #[test]
    fn active_enum_derive_unsigned_integers() {
        macro_rules! test_num_value_uint {
            ($ident: ident, $rs_type: expr, $db_type: expr, $col_def: ident) => {
                #[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
                #[sea_orm(rs_type = $rs_type, db_type = $db_type)]
                pub enum $ident {
                    #[sea_orm(num_value = 1)]
                    Big,
                    #[sea_orm(num_value = 0)]
                    Small,
                }

                test_uint!($ident, $rs_type, $db_type, $col_def);
            };
        }

        macro_rules! test_fallback_uint {
            ($ident: ident, $fallback_type: ident, $rs_type: expr, $db_type: expr, $col_def: ident) => {
                #[derive(Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
                #[sea_orm(rs_type = $rs_type, db_type = $db_type)]
                #[repr($fallback_type)]
                pub enum $ident {
                    Big = 1,
                    Small = 0,
                }

                test_uint!($ident, $rs_type, $db_type, $col_def);
            };
        }

        macro_rules! test_uint {
            ($ident: ident, $rs_type: expr, $db_type: expr, $col_def: ident) => {
                assert_eq!($ident::Big.to_value(), 1);
                assert_eq!($ident::Small.to_value(), 0);

                assert_eq!($ident::try_from_value(&1).ok(), Some($ident::Big));
                assert_eq!($ident::try_from_value(&0).ok(), Some($ident::Small));
                assert_eq!(
                    $ident::try_from_value(&2).err(),
                    Some(type_err(format!(
                        "unexpected value for {} enum: 2",
                        stringify!($ident)
                    )))
                );

                assert_eq!($ident::db_type(), ColumnType::$col_def.def());

                assert_eq!(format!("{}", $ident::Big), "Big");
                assert_eq!(format!("{}", $ident::Small), "Small");
            };
        }

        test_num_value_uint!(U8, "u8", "TinyInteger", TinyInteger);
        test_num_value_uint!(U16, "u16", "SmallInteger", SmallInteger);
        test_num_value_uint!(U32, "u32", "Integer", Integer);
        test_num_value_uint!(U64, "u64", "BigInteger", BigInteger);

        test_fallback_uint!(U8Fallback, u8, "u8", "TinyInteger", TinyInteger);
        test_fallback_uint!(U16Fallback, u16, "u16", "SmallInteger", SmallInteger);
        test_fallback_uint!(U32Fallback, u32, "u32", "Integer", Integer);
        test_fallback_uint!(U64Fallback, u64, "u64", "BigInteger", BigInteger);
    }

    #[test]
    fn escaped_non_uax31() {
        #[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Copy)]
        #[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "pop_os_names_typos")]
        pub enum PopOSTypos {
            #[sea_orm(string_value = "Pop!_OS")]
            PopOSCorrect,
            #[sea_orm(string_value = "Pop\u{2757}_OS")]
            PopOSEmoji,
            #[sea_orm(string_value = "Pop!_操作系统")]
            PopOSChinese,
            #[sea_orm(string_value = "PopOS")]
            PopOSASCIIOnly,
            #[sea_orm(string_value = "Pop OS")]
            PopOSASCIIOnlyWithSpace,
            #[sea_orm(string_value = "Pop!OS")]
            PopOSNoUnderscore,
            #[sea_orm(string_value = "Pop_OS")]
            PopOSNoExclaimation,
            #[sea_orm(string_value = "!PopOS_")]
            PopOSAllOverThePlace,
            #[sea_orm(string_value = "Pop!_OS22.04LTS")]
            PopOSWithVersion,
            #[sea_orm(string_value = "22.04LTSPop!_OS")]
            PopOSWithVersionPrefix,
            #[sea_orm(string_value = "!_")]
            PopOSJustTheSymbols,
            #[sea_orm(string_value = "")]
            Nothing,
            // This WILL fail:
            // Both PopOs and PopOS will create identifier "Popos"
            // #[sea_orm(string_value = "PopOs")]
            // PopOSLowerCase,
        }
        let values = [
            "Pop!_OS",
            "Pop\u{2757}_OS",
            "Pop!_操作系统",
            "PopOS",
            "Pop OS",
            "Pop!OS",
            "Pop_OS",
            "!PopOS_",
            "Pop!_OS22.04LTS",
            "22.04LTSPop!_OS",
            "!_",
            "",
        ];
        for (variant, val) in PopOSTypos::iter().zip(values) {
            assert_eq!(variant.to_value(), val);
            assert_eq!(PopOSTypos::try_from_value(&val.to_owned()), Ok(variant));
        }

        #[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, DeriveDisplay)]
        #[sea_orm(
            rs_type = "String",
            db_type = "String(None)",
            enum_name = "conflicting_string_values"
        )]
        pub enum ConflictingStringValues {
            #[sea_orm(string_value = "")]
            Member1,
            #[sea_orm(string_value = "$")]
            Member2,
            #[sea_orm(string_value = "$$")]
            Member3,
            #[sea_orm(string_value = "AB")]
            Member4,
            #[sea_orm(string_value = "A_B")]
            Member5,
            #[sea_orm(string_value = "A$B")]
            Member6,
            #[sea_orm(string_value = "0 123")]
            Member7,
        }
        type EnumVariant = ConflictingStringValuesVariant;
        assert_eq!(EnumVariant::__Empty.to_string(), "");
        assert_eq!(EnumVariant::_0x24.to_string(), "$");
        assert_eq!(EnumVariant::_0x240x24.to_string(), "$$");
        assert_eq!(EnumVariant::Ab.to_string(), "AB");
        assert_eq!(EnumVariant::A0x5Fb.to_string(), "A_B");
        assert_eq!(EnumVariant::A0x24B.to_string(), "A$B");
        assert_eq!(EnumVariant::_0x300x20123.to_string(), "0 123");
    }

    #[test]
    fn test_derive_display() {
        use crate::DeriveDisplay;

        #[derive(DeriveDisplay)]
        enum DisplayTea {
            EverydayTea,
            #[sea_orm(display_value = "Breakfast Tea")]
            BreakfastTea,
        }
        assert_eq!(format!("{}", DisplayTea::EverydayTea), "EverydayTea");
        assert_eq!(format!("{}", DisplayTea::BreakfastTea), "Breakfast Tea");
    }
}
