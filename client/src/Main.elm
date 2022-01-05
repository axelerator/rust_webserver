module Main exposing (..)

import Api
import Browser
import Html exposing (Html, button, div, input, span, text)
import Html.Events exposing (onClick)
import Pages.Chat as Chat
import Pages.Login as Login
import String exposing (fromInt)



-- MAIN


main =
    Browser.element
        { init = init
        , update = update
        , subscriptions = subscriptions
        , view = view
        }



-- MODEL


type Model
    = OnLogin Login.Model
    | OnChat Chat.Model


init : () -> ( Model, Cmd Msg )
init _ =
    ( OnLogin Login.init
    , Cmd.none
    )



-- UPDATE


type Msg
    = ForLogin Login.Msg
    | ForChat Chat.Msg


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model ) of
        ( ForLogin ((Login.GotLoginResponse httpResponse) as subMsg), OnLogin subModel ) ->
            let
                loginSuccessModel =
                    case httpResponse of
                        Ok loginResponse ->
                            case loginResponse of
                                Api.LoginSuccess { token } ->
                                    Just <| OnChat <| Chat.fromTokenAndUsername token "placeholder"

                                _ ->
                                    Nothing

                        _ ->
                            Nothing
            in
            case loginSuccessModel of
                Just chatModel ->
                    ( chatModel, Cmd.none )

                Nothing ->
                    let
                        ( updateSubModel, cmd ) =
                            Login.update subMsg subModel
                    in
                    ( OnLogin updateSubModel
                    , Cmd.map ForLogin cmd
                    )

        ( ForLogin subMsg, OnLogin subModel ) ->
            let
                ( updateSubModel, cmd ) =
                    Login.update subMsg subModel
            in
            ( OnLogin updateSubModel
            , Cmd.map ForLogin cmd
            )

        _ ->
            ( model, Cmd.none )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.none



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ div [] [ text "LOOK MUM, NO SERVER!!8" ]
        , case model of
            OnLogin subModel ->
                Html.map ForLogin <| Login.view subModel

            OnChat subModel ->
                Html.map ForChat <| Chat.view subModel
        ]
