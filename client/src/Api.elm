module Api exposing (..)

import Json.Decode as Decode exposing (Decoder, field)
import Json.Encode as Encode exposing (Value)


type alias Login =
    { username : String
    , password : String
    }


type LoginResponse
    = LoginSuccess SuccessDetails
    | LoginFailure FailureDetails


type alias SuccessDetails =
    { token : String }


type alias FailureDetails =
    { msg : String }


loginEncoder : Login -> Value
loginEncoder login =
    Encode.object
        [ ( "username", Encode.string login.username )
        , ( "password", Encode.string login.password )
        ]


decodeLoginResponse : Decoder LoginResponse
decodeLoginResponse =
    Decode.oneOf [ decodeLoginSuccess, decodeLoginFailure ]


decodeLoginSuccess : Decoder LoginResponse
decodeLoginSuccess =
    Decode.map LoginSuccess
        (field "Success" decodeSuccessDetails)


decodeSuccessDetails : Decoder SuccessDetails
decodeSuccessDetails =
    Decode.map SuccessDetails
        (field "token" Decode.string)


decodeLoginFailure : Decoder LoginResponse
decodeLoginFailure =
    Decode.map LoginFailure
        (field "Failure" decodeFailureDetails)


decodeFailureDetails : Decoder FailureDetails
decodeFailureDetails =
    Decode.map FailureDetails
        (field "msg" Decode.string)
