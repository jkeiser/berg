#[macro_use]
pub mod compiler_test;

compiler_tests! {
    // numbers
    greater_than_0_0: "0>0" => type(false),
    greater_than_1_1: "1>1" => type(false),
    greater_than_1_0: "1>0" => type(true),
    greater_than_0_1: "0>1" => type(false),
    greater_than_1_2: "1>2" => type(false),
    greater_than_big_big: "1982648372164176312796419487198>1982648372164176312796419487198" => type(false),
    greater_than_big_big2: "1982648372164176312796419487198>99127934712648732164276347216429663" => type(false),
    greater_than_big2_big: "99127934712648732164276347216429663>1982648372164176312796419487198" => type(true),

    greater_or_equal_0_0: "0>=0" => type(true),
    greater_or_equal_1_1: "1>=1" => type(true),
    greater_or_equal_1_0: "1>=0" => type(true),
    greater_or_equal_0_1: "0>=1" => type(false),
    greater_or_equal_1_2: "1>=2" => type(false),
    greater_or_equal_big_big: "1982648372164176312796419487198>=1982648372164176312796419487198" => type(true),
    greater_or_equal_big_big2: "1982648372164176312796419487198>=99127934712648732164276347216429663" => type(false),
    greater_or_equal_big2_big: "99127934712648732164276347216429663>=1982648372164176312796419487198" => type(true),

    less_than_0_0: "0<0" => type(false),
    less_than_1_1: "1<1" => type(false),
    less_than_1_0: "1<0" => type(false),
    less_than_0_1: "0<1" => type(true),
    less_than_1_2: "1<2" => type(true),
    less_than_big_big: "1982648372164176312796419487198<1982648372164176312796419487198" => type(false),
    less_than_big_big2: "1982648372164176312796419487198<99127934712648732164276347216429663" => type(true),
    less_than_big2_big: "99127934712648732164276347216429663<1982648372164176312796419487198" => type(false),

    less_or_equal_0_0: "0<=0" => type(true),
    less_or_equal_1_1: "1<=1" => type(true),
    less_or_equal_1_0: "1<=0" => type(false),
    less_or_equal_0_1: "0<=1" => type(true),
    less_or_equal_1_2: "1<=2" => type(true),
    less_or_equal_big_big: "1982648372164176312796419487198<=1982648372164176312796419487198" => type(true),
    less_or_equal_big_big2: "1982648372164176312796419487198<=99127934712648732164276347216429663" => type(true),
    less_or_equal_big2_big: "99127934712648732164276347216429663<=1982648372164176312796419487198" => type(false),
}
