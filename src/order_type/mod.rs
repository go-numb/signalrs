/// 引数で分岐するクリックイベントを定義するモジュール
/// chooseが起点となり、引数に従いそれぞれのモジュールに分岐する
pub mod choose;
pub mod entry; // as 1,2
pub mod exit; // as 3,4,5
pub mod origin; // as 99
pub mod process;
pub mod simple; // as 0 // as select
