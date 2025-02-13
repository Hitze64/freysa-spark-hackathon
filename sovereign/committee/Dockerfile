FROM --platform=linux/amd64 amazonlinux:2023

COPY setup.sh /setup.sh
RUN chmod +x /setup.sh && ./setup.sh

COPY get_committee_message.sh /get_committee_message.sh
RUN chmod +x /get_committee_message.sh

WORKDIR /workspace

ENTRYPOINT ["/get_committee_message.sh"]
