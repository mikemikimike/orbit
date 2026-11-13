{{/*
ORBIT Helm helpers.
*/}}

{{- define "orbit.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "orbit.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "orbit.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "orbit.labels" -}}
helm.sh/chart: {{ include "orbit.chart" . }}
app.kubernetes.io/name: {{ include "orbit.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: orbit
{{- end -}}

{{- define "orbit.selectorLabels" -}}
app.kubernetes.io/name: {{ include "orbit.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "orbit.componentLabels" -}}
{{ include "orbit.selectorLabels" . }}
app.kubernetes.io/component: {{ .component }}
{{- end -}}

{{- define "orbit.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (printf "%s" (include "orbit.fullname" .)) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}

{{- define "orbit.secretName" -}}
{{- default (printf "%s-secrets" (include "orbit.fullname" .)) .Values.secrets.existingSecret -}}
{{- end -}}

{{- define "orbit.databaseUrl" -}}
{{- if .Values.secrets.databaseUrl -}}
{{- .Values.secrets.databaseUrl -}}
{{- else if .Values.externalDatabase.enabled -}}
{{- required "externalDatabase.url is required when externalDatabase.enabled=true" .Values.externalDatabase.url -}}
{{- else -}}
{{- printf "postgres://%s:%s@%s-postgresql:5432/%s?sslmode=disable" .Values.postgresql.user .Values.postgresql.password (include "orbit.fullname" .) .Values.postgresql.database -}}
{{- end -}}
{{- end -}}

{{- define "orbit.natsUrl" -}}
{{- if .Values.externalNats.enabled -}}
{{- required "externalNats.url is required when externalNats.enabled=true" .Values.externalNats.url -}}
{{- else -}}
{{- printf "nats://%s-nats:4222" (include "orbit.fullname" .) -}}
{{- end -}}
{{- end -}}

{{/*
Image reference. The tag defaults to the chart appVersion so a released chart always
pulls the images built for that release. Override per component with `<component>.tag`.
Call with: dict "root" $ "image" .Values.api.image "registry" .Values.imageRegistry "tag" .Values.api.tag
*/}}
{{- define "orbit.image" -}}
{{- $registry := trimSuffix "/" .registry -}}
{{- $tag := .tag -}}
{{- if not $tag -}}
{{- $tag = .root.Chart.AppVersion -}}
{{- end -}}
{{- if $registry -}}
{{- printf "%s/%s:%s" $registry .image $tag -}}
{{- else -}}
{{- printf "%s:%s" .image $tag -}}
{{- end -}}
{{- end -}}

{{- define "orbit.apiHost" -}}
{{- printf "%s-api" (include "orbit.fullname" .) -}}
{{- end -}}

{{- define "orbit.webHost" -}}
{{- printf "%s-web" (include "orbit.fullname" .) -}}
{{- end -}}
