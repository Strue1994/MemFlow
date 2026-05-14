{{- define "memflow.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "memflow.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{-  := default .Chart.Name .Values.nameOverride }}
{{- if contains  .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name  | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{- define "memflow.labels" -}}
helm.sh/chart: {{ include "memflow.name" . }}-{{ .Chart.Version | replace "+" "_" }}
{{ include "memflow.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "memflow.selectorLabels" -}}
app.kubernetes.io/name: {{ include "memflow.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "memflow.imagePullSecret" -}}
{{- with .Values.global.imagePullSecrets }}
imagePullSecrets:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}
