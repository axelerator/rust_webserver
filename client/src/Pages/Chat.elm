module Pages.Chat exposing (Model, Msg, fromTokenAndUsername, init, mapEvent, update, view)

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
                clientState =
                    case e of
                        UpdateGameState { client_state } ->
                            Just client_state

                        _ ->
                            model.clientState

                model_ =
                    { model
                        | events = e :: model.events
                        , clientState = clientState
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


init =
    { currentChannel = "Home" }


eventToString e =
    case e of
        HelloClient ->
            "HelloClient"

        UpdateGameState _ ->
            "UpdateGameState"

        AvailableRounds _ ->
            "AvailableRounds"


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


viewGame state =
    case state of
        Lobby { playerCount, playerReadyCount } ->
            div []
                [ text "players "
                , text <| String.fromInt playerReadyCount
                , text " of "
                , text <| String.fromInt playerCount
                , text " are ready"
                ]

        InLevel { currentInstruction } ->
            div [] [ text currentInstruction ]
