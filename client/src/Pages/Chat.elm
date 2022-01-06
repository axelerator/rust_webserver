module Pages.Chat exposing (Model, Msg, fromTokenAndUsername, init, mapEvent, update, view)

import Api
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
    }


fromTokenAndUsername token username =
    { currentChannel = "a channel name"
    , session =
        { username = username
        , token = token
        }
    , events = []
    }


type ToClientEnvelope
    = SuperSeeded
    | AppMsg ToClient


type ToClient
    = HelloClient


mapEvent : Value -> Maybe Msg
mapEvent value =
    case Decode.decodeValue eventDecoder value of
        Ok SuperSeeded ->
            Nothing

        Ok (AppMsg event) ->
            Just <| GotEvent event

        Err error ->
            Just <| EventDecoderError <| Decode.errorToString error


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
    Decode.string
        |> Decode.andThen
            (\s ->
                case s of
                    "HelloClient" ->
                        Decode.succeed HelloClient

                    _ ->
                        Decode.fail <| "Unkown ToClient: " ++ s
            )


type Msg
    = NoOp
    | EventDecoderError String
    | GotEvent ToClient
    | SendAction


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )

        EventDecoderError e ->
            ( Debug.log e model, Cmd.none )

        GotEvent e ->
            ( { model | events = e :: model.events }, Cmd.none )

        SendAction ->
            ( model, sendAction model.session.token )


sendAction : String -> Cmd Msg
sendAction token =
    Http.post
        { url = "/action"
        , body = Http.jsonBody <| Api.toBackendEnvelopeEncoder { token = token }
        , expect = Http.expectWhatever (\_ -> NoOp)
        }


init =
    { currentChannel = "Home" }


eventToString e =
    case e of
        HelloClient ->
            "HelloClient"


view : Model -> Html Msg
view model =
    div []
        [ text model.currentChannel
        , ul [] <| List.map (\e -> li [] [ text <| eventToString e ]) model.events
        , button [ onClick SendAction ] [ text "send" ]
        ]
