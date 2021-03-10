tar -czvf starscape-web.tar.gz ../web/dist/
scp ./starscape-web.tar.gz root@8.9.36.49:/root
# On server:
# tar -xvf starscape-web.tar.gz
# rm -Rf /root/starscape/web/dist
# mv web/dist/ starscape/web/
