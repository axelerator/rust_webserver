module Pages.Login exposing (Model, Msg(..), init, update, view)

import Api exposing (LoginResponse(..))
import Html exposing (Html, button, div, input, label, text)
import Html.Attributes exposing (type_, value)
import Html.Events exposing (onClick, onInput)
import Http


type alias Model =
    { username : String
    , password : String
    , loading : Bool
    }


init =
    { username = "at"
    , password = "aa"
    , loading = False
    }


type Msg
    = ChangeUsername String
    | ChangePassword String
    | AttemptLogin String String
    | GotLoginResponse (Result Http.Error Api.LoginResponse)


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        ChangeUsername s ->
            ( { model | username = s }
            , Cmd.none
            )

        ChangePassword s ->
            ( { model | password = s }
            , Cmd.none
            )

        AttemptLogin username password ->
            ( { model | loading = False }
            , attemptLogin username password
            )

        GotLoginResponse httpResponse ->
            case httpResponse of
                Ok loginResponse ->
                    case loginResponse of
                        LoginSuccess { token } ->
                            ( { model | username = token, loading = False }
                            , Cmd.none
                            )

                        LoginFailure failure ->
                            ( { model | username = failure.msg, loading = False }
                            , Cmd.none
                            )

                Err err ->
                    ( model
                    , Cmd.none
                    )


attemptLogin : String -> String -> Cmd Msg
attemptLogin username password =
    Http.post
        { url = "/login"
        , body = Http.jsonBody <| Api.loginEncoder <| { username = username, password = password }
        , expect = Http.expectJson GotLoginResponse Api.decodeLoginResponse
        }


view : Model -> Html Msg
view model =
    div []
        [ inp "username" "text" model.username ChangeUsername
        , inp "password" "password" model.password ChangePassword
        , if model.loading then
            text "loading"

          else
            button [ onClick <| AttemptLogin model.username model.password ] [ text "login" ]
        ]


inp : String -> String -> String -> (String -> Msg) -> Html Msg
inp labelTxt tpe val msg =
    div []
        [ label [] [ text labelTxt ]
        , input
            [ value val
            , onInput msg
            , type_ tpe
            ]
            []
        ]
