
.PHONY: build run

build:
	docker build -t poll .

run:
	docker run -p 80:8080 --name poll --volume=$(pwd)/db:/usr/local/polls/db --rm -it poll