module Api exposing (..)

import Http exposing (Error(..))
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


type ToBackend
    = Init
    | StartGame
    | ToggleReady
    | ChangeSetting ItemId
    | GetAvailableRounds
    | JoinGame RoundId


type alias ToBackendEnvelope =
    { token : String
    , toBackend : ToBackend
    }


toBackendEnvelopeEncoder : ToBackendEnvelope -> Value
toBackendEnvelopeEncoder be =
    Encode.object
        [ ( "token", Encode.string be.token )
        , ( "to_backend", encodeToBackend be.toBackend )
        ]


encodeToBackend : ToBackend -> Value
encodeToBackend tb =
    case tb of
        Init ->
            Encode.string "Init"

        StartGame ->
            Encode.string "StartGame"

        ToggleReady ->
            Encode.string "ToggleReady"

        ChangeSetting item_id ->
            Encode.object
                [ ( "ChangeSetting", Encode.object [ ( "item_id", Encode.int item_id ) ] )
                ]

        GetAvailableRounds ->
            Encode.string "GetAvailableRounds"

        JoinGame roundId ->
            Encode.object
                [ ( "JoinGame", Encode.object [ ( "round_id", Encode.string roundId ) ] )
                ]


type ClientState
    = Lobby LobbyDetails
    | InLevel InLevelDetails


type alias LobbyDetails =
    { playerCount : Int, playerReadyCount : Int }


type alias InLevelDetails =
    { currentInstruction : String
    , uiItems : List UiItem
    , instructionsExecuted : Int
    , instructionsMissed : Int
    }


type alias ItemId =
    Int


type alias UiItem =
    { id : ItemId
    , label : String
    , state : Int
    }


decodeClientState : Decoder ClientState
decodeClientState =
    Decode.oneOf [ decodeInLobby, decodeInLevel ]


decodeInLobby : Decoder ClientState
decodeInLobby =
    let
        details =
            Decode.map2 LobbyDetails
                (field "player_count" Decode.int)
                (field "player_ready_count" Decode.int)
    in
    Decode.map Lobby
        (field "Lobby" details)


decodeInLevel : Decoder ClientState
decodeInLevel =
    let
        details =
            Decode.map4 InLevelDetails
                (field "current_instruction" Decode.string)
                (field "ui_items" (Decode.list decodeUiItem))
                (field "instructions_executed" Decode.int)
                (field "instructions_missed" Decode.int)
    in
    Decode.map InLevel
        (field "InGame" details)


decodeUiItem : Decoder UiItem
decodeUiItem =
    Decode.map3 UiItem
        (field "id" Decode.int)
        (field "label" Decode.string)
        (field "state" Decode.int)


type ToClientEnvelope
    = SuperSeeded
    | AppMsg ToClient


type alias RoundId =
    String


type ToClient
    = HelloClient
    | UpdateGameState UpdateGameStateDetails
    | AvailableRounds AvailableRoundsDetails
    | EnterRound UpdateGameStateDetails


type alias AvailableRoundsDetails =
    { roundIds : List RoundId }


type alias UpdateGameStateDetails =
    { clientState : ClientState }


eventDecoder : Decoder ToClientEnvelope
eventDecoder =
    Decode.oneOf [ superSeededDecoder, appMsgDecoder ]


superSeededDecoder : Decoder ToClientEnvelope
superSeededDecoder =
    Decode.field "SuperSeeded" (Decode.list Decode.string)
        |> Decode.andThen
            (\_ -> Decode.succeed SuperSeeded)


appMsgDecoder : Decoder ToClientEnvelope
appMsgDecoder =
    Decode.map AppMsg
        (Decode.field "AppMsg" toClientDecoder)


toClientDecoder : Decoder ToClient
toClientDecoder =
    Decode.oneOf [ decodeHelloClient, decodeUpdateGameState, decodeAvailableRounds, decodeEnterRound ]


decodeHelloClient : Decoder ToClient
decodeHelloClient =
    Decode.string
        |> Decode.andThen
            (\s ->
                case s of
                    "HelloClient" ->
                        Decode.succeed HelloClient

                    _ ->
                        Decode.fail <| "Unkown ToClient: " ++ s
            )


decodeUpdateGameState : Decoder ToClient
decodeUpdateGameState =
    let
        updateGameStateDetailsDecoder =
            Decode.map UpdateGameStateDetails
                (field "client_state" decodeClientState)
    in
    Decode.map UpdateGameState
        (field "UpdateGameState" updateGameStateDetailsDecoder)


decodeEnterRound : Decoder ToClient
decodeEnterRound =
    let
        updateGameStateDetailsDecoder =
            Decode.map UpdateGameStateDetails
                (field "client_state" decodeClientState)
    in
    Decode.map EnterRound
        (field "EnterRound" updateGameStateDetailsDecoder)


decodeAvailableRounds : Decoder ToClient
decodeAvailableRounds =
    let
        availableRoundsDetailsDecoder =
            Decode.map AvailableRoundsDetails
                (field "round_ids" <| Decode.list Decode.string)
    in
    Decode.map AvailableRounds
        (field "AvailableRounds" availableRoundsDetailsDecoder)


sendAction : (Result Http.Error () -> msg) -> String -> ToBackend -> Cmd msg
sendAction actionConfirmationHandler token toBackend =
    Http.post
        { url = "/action"
        , body = Http.jsonBody <| toBackendEnvelopeEncoder { token = token, toBackend = toBackend }
        , expect = Http.expectWhatever actionConfirmationHandler
        }


httpErrorToString : Http.Error -> String
httpErrorToString error =
    case error of
        BadUrl url ->
            "The URL " ++ url ++ " was invalid"

        Timeout ->
            "Unable to reach the server, try again"

        NetworkError ->
            "Unable to reach the server, check your network connection"

        BadStatus 500 ->
            "The server had a problem, try again later"

        BadStatus 400 ->
            "Verify your information and try again"

        BadStatus _ ->
            "Unknown error"

        BadBody errorMessage ->
            errorMessage
