FROM scratch
COPY mkube-console /usr/local/bin/mkube-console
COPY static/ /static/
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/mkube-console"]
CMD ["/etc/mkube-console/config.yaml"]
