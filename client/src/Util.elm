module Util exposing (..)

import Css exposing (backgroundImage, backgroundPosition, backgroundPosition2, height, px, width)
import Html.Styled exposing (Html, div)
import Html.Styled.Attributes exposing (css)


type alias KnobDefinition =
    { filename : String
    , maxValue : Int
    , w : Int
    , h : Int
    }


knobDefinitions =
    [ { filename = "2_100x100.png", maxValue = 2, w = 100, h = 100 }
    , { filename = "3-100x100.png", maxValue = 3, w = 100, h = 100 }
    , { filename = "4_100x25.png", maxValue = 4, w = 100, h = 25 }
    , { filename = "5_128x32.png", maxValue = 5, w = 128, h = 32 }
    , { filename = "6_100x100.png", maxValue = 6, w = 100, h = 100 }
    , { filename = "7_100x100.png", maxValue = 7, w = 100, h = 100 }
    , { filename = "8_100x100.png", maxValue = 8, w = 100, h = 100 }
    , { filename = "9_128x32.png", maxValue = 9, w = 128, h = 32 }
    , { filename = "10_100x100.png", maxValue = 10, w = 100, h = 100 }
    ]


knob : KnobDefinition -> Int -> Html msg
knob { filename, w, h } value =
    div
        [ css
            [ width <| px <| toFloat w
            , height <| px <| toFloat h
            , backgroundImage <| Css.url <| "/assets/knobs/" ++ filename
            , backgroundPosition2 (px 0) (px <| toFloat (-h * value))
            ]
        ]
        []
