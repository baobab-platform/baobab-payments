# Gate D1 test services (issuer, processor). CI and local integration only.
FROM mirror.gcr.io/library/python:3.12-slim
COPY wheels /wheels
RUN pip install --no-index --find-links /wheels cryptography && rm -rf /wheels
COPY issuer/issuer.py processor/processor.py /app/
USER 65532:65532
