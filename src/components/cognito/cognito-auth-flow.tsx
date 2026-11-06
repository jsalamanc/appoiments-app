"use client";

import { useState } from "react";

import { CognitoConfirmForm } from "./confirm-form";
import { CognitoForgotPasswordForm } from "./forgot-password-form";
import { CognitoLoginForm } from "./login-form";
import { CognitoResetPasswordForm } from "./reset-password-form";
import { CognitoSignupForm } from "./signup-form";

type FlowStage = "signup" | "confirm" | "login" | "forgot" | "reset";

export type CognitoAuthFlowCopy = {
  loginSubtitle?: string;
};

export type CognitoAuthFlowProps = {
  /** Pantalla con la que arranca el flujo. Default "login". */
  initialStage?: "login" | "signup";
  /** Se dispara apenas `cognito_login` termina bien. */
  onAuthenticated: () => void;
  /** Overrides de texto para distintos contextos (gate inicial vs. Ajustes). */
  copy?: CognitoAuthFlowCopy;
};

/**
 * Encapsula la máquina de estados signup → confirm → login → forgot → reset
 * de la cuenta de Aura Studio (Cognito) — usada tanto por el gate inicial de
 * `src/app/page.tsx` (instalación nueva) como por Ajustes → Nube (una
 * instalación existente que recién ahora quiere vincular su cuenta). La
 * lógica de envío/validación vive en cada formulario, sin cambios; esto solo
 * decide cuál mostrar.
 */
export function CognitoAuthFlow({
  initialStage = "login",
  onAuthenticated,
  copy,
}: CognitoAuthFlowProps) {
  const [stage, setStage] = useState<FlowStage>(initialStage);
  const [pendingEmail, setPendingEmail] = useState("");

  if (stage === "signup") {
    return (
      <CognitoSignupForm
        onNeedsConfirmation={(email) => {
          setPendingEmail(email);
          setStage("confirm");
        }}
        onConfirmedDirectly={(email) => {
          setPendingEmail(email);
          setStage("login");
        }}
        onSwitchToLogin={() => setStage("login")}
      />
    );
  }

  if (stage === "confirm") {
    return (
      <CognitoConfirmForm
        email={pendingEmail}
        onConfirmed={(email) => {
          setPendingEmail(email);
          setStage("login");
        }}
        onChangeEmail={() => setStage("signup")}
      />
    );
  }

  if (stage === "forgot") {
    return (
      <CognitoForgotPasswordForm
        prefillEmail={pendingEmail}
        onCodeSent={(email) => {
          setPendingEmail(email);
          setStage("reset");
        }}
        onBackToLogin={() => setStage("login")}
      />
    );
  }

  if (stage === "reset") {
    return (
      <CognitoResetPasswordForm
        email={pendingEmail}
        onReset={() => setStage("login")}
      />
    );
  }

  return (
    <CognitoLoginForm
      prefillEmail={pendingEmail}
      onLoggedIn={onAuthenticated}
      onSwitchToSignup={() => setStage("signup")}
      onForgotPassword={() => setStage("forgot")}
      subtitle={copy?.loginSubtitle}
    />
  );
}
