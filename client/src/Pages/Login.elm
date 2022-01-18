module Pages.Login exposing (Model, Msg(..), init, update, view)

import Api exposing (LoginResponse(..), ToBackend(..))
import Html.Styled exposing (Html, button, div, input, label, text)
import Html.Styled.Attributes exposing (type_, value)
import Html.Styled.Events exposing (onClick, onInput)
import Http


type alias Model =
    { username : String
    , password : String
    , loading : Bool
    , msg : Maybe String
    }


init : Maybe String -> Model
init msg =
    { username = "at"
    , password = "aa"
    , loading = False
    , msg = msg
    }


type Msg
    = ChangeUsername String
    | ChangePassword String
    | AttemptLogin String String
    | GotLoginResponse (Result Http.Error Api.LoginResponse)
    | CouldNotSendAction (Result Http.Error ())


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
            ( { model | loading = True }
            , attemptLogin username password
            )

        CouldNotSendAction _ ->
            ( { model | msg = Just "Could not send action" }
            , Cmd.none
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
                            ( { model | msg = Just failure.msg, loading = False }
                            , Cmd.none
                            )

                Err err ->
                    ( { model | msg = Just <| Api.httpErrorToString err }
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
        [ case model.msg of
            Just msg ->
                text msg

            Nothing ->
                text ""
        , inp "username" "text" model.username ChangeUsername
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
