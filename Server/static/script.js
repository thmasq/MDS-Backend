// Add an event listener to the input field to handle user input
document.querySelector('#searchQuery').addEventListener('input', () => {
    // Delay the search by a short time to allow the user to finish typing
    clearTimeout(timeout);
    timeout = setTimeout(performSearch, 300); // Adjust the delay time as needed
});

// Initialize a timeout variable
let timeout;

function performSearch() {
    const searchQuery = document.querySelector('#searchQuery').value;
    const resultsContainer = document.querySelector('#results');
    const loadingIndicator = document.createElement('p');
    loadingIndicator.textContent = 'Searching...';

    // Display the loading indicator
    resultsContainer.innerHTML = '';
    resultsContainer.appendChild(loadingIndicator);

    // Send a GET request to your Actix backend
    fetch(`/search?q=${encodeURIComponent(searchQuery)}`)
        .then((response) => response.json())
        .then((data) => {
            resultsContainer.innerHTML = ''; // Clear the loading indicator

            // Loop through the results and create HTML elements to display each movie
            data.results.forEach((movie) => {
                const movieElement = document.createElement('div');
                movieElement.classList.add('movie');

                // Create elements for each field of the Movie struct
                const titleElement = document.createElement('h2');
                titleElement.textContent = movie.title;

                const posterElement = document.createElement('img');
                posterElement.src = movie.poster;

                const overviewElement = document.createElement('p');
                overviewElement.textContent = movie.overview;

                const releaseDateElement = document.createElement('p');
                releaseDateElement.textContent = `Release Date: ${new Date(
                    movie.release_date * 1000
                ).toDateString()}`;

                // Append elements to the movie container
                movieElement.appendChild(titleElement);
                movieElement.appendChild(posterElement);
                movieElement.appendChild(overviewElement);
                movieElement.appendChild(releaseDateElement);

                // Append the movie container to the results container
                resultsContainer.appendChild(movieElement);
            });
        })
        .catch((error) => {
            console.error('Error:', error);
            resultsContainer.innerHTML = 'An error occurred.';
        });
}
