module Pages.Round exposing (Model, Msg, fromEnterRound, fromTokenAndUsername, mapEvent, update, view)

import Api exposing (ClientState(..), LobbyDetails, ToBackend(..), ToClient(..), ToClientEnvelope(..), eventDecoder)
import Html exposing (Html, button, div, li, text, ul)
import Html.Events exposing (onClick)
import Http
import Json.Decode as Decode exposing (Decoder)
import Json.Encode exposing (Value)
import Session exposing (Session)


type alias Model =
    { currentChannel : String
    , session : Session
    , events : List ToClient
    , clientState : Maybe ClientState
    }


fromTokenAndUsername token username =
    { currentChannel = "a channel name"
    , session =
        { username = username
        , token = token
        }
    , events = []
    , clientState = Nothing
    }


fromEnterRound : Session -> ClientState -> Model
fromEnterRound session clientState =
    { currentChannel = "a channel name"
    , session = session
    , events = []
    , clientState = Just clientState
    }


mapEvent : Value -> Maybe Msg
mapEvent value =
    case Decode.decodeValue eventDecoder value of
        Ok SuperSeeded ->
            Nothing

        Ok (AppMsg event) ->
            Just <| GotEvent event

        Err error ->
            Just <| EventDecoderError <| Decode.errorToString error


type Msg
    = NoOp
    | EventDecoderError String
    | GotEvent ToClient
    | SendAction ToBackend


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )

        EventDecoderError e ->
            ( Debug.log e model, Cmd.none )

        GotEvent e ->
            let
                newClientState =
                    case e of
                        UpdateGameState { clientState } ->
                            Just clientState

                        _ ->
                            model.clientState

                model_ =
                    { model
                        | events = e :: model.events
                        , clientState = newClientState
                    }
            in
            ( model_, Cmd.none )

        SendAction toBackend ->
            ( model, sendAction model.session.token toBackend )


sendAction : String -> ToBackend -> Cmd Msg
sendAction token toBackend =
    Http.post
        { url = "/action"
        , body = Http.jsonBody <| Api.toBackendEnvelopeEncoder { token = token, toBackend = toBackend }
        , expect = Http.expectWhatever (\_ -> NoOp)
        }


eventToString e =
    case e of
        HelloClient ->
            "HelloClient"

        UpdateGameState _ ->
            "UpdateGameState"

        AvailableRounds _ ->
            "AvailableRounds"

        EnterRound _ ->
            "EnterRound"


view : Model -> Html Msg
view model =
    div []
        [ text model.currentChannel
        , ul [] <| List.map (\e -> li [] [ text <| eventToString e ]) model.events
        , case model.clientState of
            Nothing ->
                text "waiting"

            Just state ->
                viewGame state
        ]


viewGame client_state =
    case client_state of
        Lobby { playerCount, playerReadyCount } ->
            div []
                [ text "players "
                , text <| String.fromInt playerReadyCount
                , text " of "
                , text <| String.fromInt playerCount
                , text " are ready"
                , button [ onClick <| SendAction ToggleReady ] [ text "Ready" ]
                ]

        InLevel { currentInstruction, uiItems } ->
            let
                mkUiItem { label, state, id } =
                    li []
                        [ text label
                        , text " is "
                        , button [ onClick <| SendAction <| ChangeSetting id ]
                            [ text <|
                                if state then
                                    "ON"

                                else
                                    "OFF"
                            ]
                        ]
            in
            div []
                [ text currentInstruction
                , ul [] <| List.map mkUiItem uiItems
                ]
